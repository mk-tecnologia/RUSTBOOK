<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { getServerHealth, type ServerHealth } from '../../api/serverSettings'
import { serverHealthAction, serverHealthItems, serverHealthSummary } from '../../utils/serverHealth'
const health = ref<ServerHealth>()
const busy = ref(false)
const error = ref('')
const checkedAt = ref('')
const items = computed(() => health.value ? serverHealthItems(health.value) : [])
const summary = computed(() => serverHealthSummary(items.value, health.value?.report.warnings || []))
const colors = { ok: 'bg-green-100 text-green-800 dark:bg-green-900/25 dark:text-green-300', warning: 'bg-amber-100 text-amber-800 dark:bg-amber-900/25 dark:text-amber-300', missing: 'bg-amber-100 text-amber-800 dark:bg-amber-900/25 dark:text-amber-300', error: 'bg-red-100 text-red-800 dark:bg-red-900/25 dark:text-red-300' }
async function refresh() {
  if (busy.value) return
  busy.value = true; error.value = ''; health.value = undefined
  try { health.value = await getServerHealth(); checkedAt.value = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }) }
  catch (e) { error.value = e instanceof Error ? e.message : 'Não foi possível verificar o servidor.' }
  finally { busy.value = false }
}
onMounted(refresh)
</script>
<template>
  <section class="mb-6 rounded-xl border border-gray-200 bg-white p-5 dark:border-rd-border dark:bg-rd-card" aria-label="Estado da configuração do servidor" :aria-busy="busy">
    <header class="flex flex-wrap items-start justify-between gap-4">
      <div><h2 class="font-semibold">Estado do servidor</h2><p class="mt-1 text-sm text-gray-500 dark:text-rd-text-secondary">Configuração, conectividade e pendências para os clientes RustDesk.</p></div>
      <div class="flex flex-wrap items-center gap-2"><RouterLink to="/server-settings" class="ui-button">Configurar servidor</RouterLink><button type="button" :disabled="busy" class="ui-button" @click="refresh">{{ busy ? 'Verificando…' : 'Verificar agora' }}</button></div>
    </header>
    <p v-if="busy" role="status" class="mt-5 text-sm text-gray-500">Consultando os endereços salvos e o agente do servidor…</p>
    <p v-if="error" role="alert" class="mt-5 rounded-lg bg-red-50 p-3 text-sm text-red-700 dark:bg-red-950/20 dark:text-red-300">Não foi possível confirmar o estado do servidor. {{ error }} Tente Verificar agora.</p>
    <template v-if="health">
      <div class="my-5 flex flex-wrap items-center justify-between gap-3"><span role="status" class="rounded-full px-3 py-1 text-xs font-medium" :class="colors[summary.state as keyof typeof colors]">{{ summary.label }}</span><span class="text-xs text-gray-500">Verificado às {{ checkedAt }}</span></div>
      <dl class="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
        <div v-for="item in items" :key="item.key" class="min-w-0 rounded-lg border border-gray-100 p-3 dark:border-rd-border">
          <dt class="mb-2 text-sm font-medium">{{ item.label }}</dt><dd><span class="inline-flex items-center gap-1.5 rounded-full px-2 py-1 text-xs" :class="colors[item.state]"><span aria-hidden="true" class="size-1.5 rounded-full bg-current" />{{ item.status }}</span><p class="mt-2 break-words text-xs leading-relaxed text-gray-500 dark:text-rd-text-secondary">{{ item.detail }}</p><RouterLink v-if="item.state !== 'ok'" :to="serverHealthAction(item.key, health).to" class="mt-3 inline-flex items-center gap-1 text-xs font-medium text-rd-primary hover:underline">{{ serverHealthAction(item.key, health).label }} <span aria-hidden="true">→</span></RouterLink></dd>
        </div>
      </dl>
      <ul v-if="health.report.warnings.length" class="mt-4 list-inside list-disc space-y-1 text-xs text-amber-700 dark:text-amber-300"><li v-for="warning in health.report.warnings" :key="warning">{{ warning }}</li></ul>
      <RouterLink v-if="health.report.settings.api_url.startsWith('http://')" to="/server-settings#server-https" class="mt-3 inline-flex items-center gap-1 text-xs font-medium text-rd-primary hover:underline">Configurar HTTPS <span aria-hidden="true">→</span></RouterLink>
      <p class="mt-4 text-xs leading-relaxed text-gray-500 dark:text-rd-text-secondary">As consultas partem da agenda. TCP acessível não confirma TLS, autenticação do relay ou acesso externo. Valide uma conexão real entre clientes externos após alterar a rede.</p>
    </template>
  </section>
</template>
