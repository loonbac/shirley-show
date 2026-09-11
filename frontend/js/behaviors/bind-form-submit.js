/**
 * Binds submit handler to a form with busy lifecycle handling.
 * @param {HTMLFormElement} formEl 
 * @param {(event: SubmitEvent) => Promise<void> | void} onSubmit 
 * @returns {() => void} cleanup function
 */
export function bindFormSubmit(formEl, onSubmit) {
  if (!formEl || typeof onSubmit !== 'function') {
    return () => {};
  }

  let isSubmitting = false;
  const submitBtn = formEl.querySelector('button[type="submit"]');

  const handleSubmit = async (event) => {
    event.preventDefault();
    if (isSubmitting) return;

    isSubmitting = true;
    if (submitBtn) {
      submitBtn.disabled = true;
      submitBtn.dataset.originalText = submitBtn.textContent;
    }

    try {
      await onSubmit(event);
    } catch (err) {
      console.error('Form submission error:', err);
    } finally {
      isSubmitting = false;
      if (submitBtn) {
        submitBtn.disabled = false;
      }
    }
  };

  formEl.addEventListener('submit', handleSubmit);

  return () => {
    formEl.removeEventListener('submit', handleSubmit);
  };
}
