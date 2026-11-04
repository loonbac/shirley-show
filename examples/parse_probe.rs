use shirley_show::infra::vocal::colab_vocal_separator::parse_execution_result;

fn main() {
    let raw = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    println!("bytes: {}", raw.len());
    match parse_execution_result(&raw) {
        Some(r) => println!(
            "OK url={} sha={} dur={} size={}",
            r.url, r.sha256, r.duration, r.size
        ),
        None => println!("NONE"),
    }
}
