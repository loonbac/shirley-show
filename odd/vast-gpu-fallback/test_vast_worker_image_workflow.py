"""Structural checks for the Vast worker image workflow.

Run from the repository root:

    python3 -m unittest discover -s odd/vast-gpu-fallback -p 'test_*.py'

These checks read the public workflow structure only: the runner disk is freed
before the build, the 13.5 GB image is stored once, and the existing smoke and
push behaviour is preserved.
"""

import json
import subprocess
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
WORKFLOW = REPO / ".github" / "workflows" / "vast-worker-image.yml"


def load_workflow():
    result = subprocess.run(
        ["npx", "--yes", "js-yaml", str(WORKFLOW)],
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


class VastWorkerImageWorkflowTest(unittest.TestCase):
    def setUp(self):
        self.doc = load_workflow()
        self.steps = self.doc["jobs"]["build"]["steps"]

    def step_named(self, name):
        matches = [step for step in self.steps if step.get("name") == name]
        self.assertEqual(len(matches), 1, f"expected one step named {name!r}")
        return matches[0]

    def test_first_step_frees_runner_disk_without_third_party_actions(self):
        first = self.steps[0]
        self.assertNotIn("uses", first)
        run = first["run"]
        for path in (
            "/usr/share/dotnet",
            "/usr/local/lib/android",
            "/opt/ghc",
            "/opt/hostedtoolcache/CodeQL",
            "/usr/local/share/boost",
            '"$AGENT_TOOLSDIRECTORY"',
        ):
            self.assertIn(path, run, f"cleanup misses {path}")
        self.assertIn("sudo docker image prune -af", run)
        self.assertIn("df -h /", run)

    def test_image_is_built_once_without_a_second_copy(self):
        self.assertFalse(
            any("setup-buildx-action" in step.get("uses", "") for step in self.steps),
            "buildx setup keeps a second image cache on the runner",
        )
        builds = [
            step
            for step in self.steps
            if "docker build" in step.get("run", "") or "build-push-action" in step.get("uses", "")
        ]
        self.assertEqual(len(builds), 1, "the image must be built exactly once")
        build = builds[0]
        self.assertNotIn("uses", build, "a build action adds a separate buildx cache")
        run = build["run"]
        self.assertIn("docker build", run)
        self.assertIn("--file docker/vast-worker/Dockerfile", run)
        for tag in ("${IMAGE}:${GITHUB_SHA}", "${IMAGE}:latest"):
            self.assertIn(f"--tag {tag}", run)
        self.assertNotIn("docker load", run)
        self.assertNotIn("load: true", run)
        self.assertNotIn("push: true", run)

    def test_disk_evidence_is_reported_after_the_build(self):
        names = [step.get("name") for step in self.steps]
        build_steps = [
            i for i, step in enumerate(self.steps) if "docker build" in step.get("run", "")
        ]
        self.assertTrue(build_steps, "no docker build step found")
        build_index = build_steps[0]
        evidence = [
            step["run"]
            for step in self.steps[build_index + 1 :]
            if "df -h /" in step.get("run", "")
        ]
        self.assertTrue(evidence, f"no df -h / after the build in {names}")

    def test_existing_smoke_and_push_behaviour_is_preserved(self):
        self.assertEqual(self.doc["permissions"]["packages"], "write")
        smoke = self.step_named("Smoke the separation entrypoint")["run"]
        self.assertIn("--network none", smoke)
        push = self.step_named("Push :sha and :latest")
        self.assertEqual(push.get("if"), "github.event_name != 'pull_request'")
        for tag in ("${IMAGE}:${GITHUB_SHA}", "${IMAGE}:latest"):
            self.assertIn(f'docker push "{tag}"', push["run"])
        login = self.step_named("Log in to GHCR")
        self.assertEqual(login.get("if"), "github.event_name != 'pull_request'")
        self.assertEqual(login["with"]["password"], "${{ secrets.GITHUB_TOKEN }}")


if __name__ == "__main__":
    unittest.main()
