import { ref } from 'vue'

export function useAsyncTask() {
  const pending = ref(false)
  const error = ref('')
  async function run(work: () => Promise<void>) {
    if (pending.value) return
    pending.value = true; error.value = ''
    try { await work() }
    catch (e) { error.value = e instanceof Error ? e.message : 'Operation failed. Please try again.' }
    finally { pending.value = false }
  }
  return { pending, error, run }
}
