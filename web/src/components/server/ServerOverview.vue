<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { getClientConfiguration, type ClientConfiguration } from '../../api/serverSettings'
import type { NativeConnection } from '../../utils/connection'
import ErrorNotice from '../ui/ErrorNotice.vue'
const emit = defineEmits<{ changed: [profile: NativeConnection] }>()
const configuration = ref<ClientConfiguration>()
const error = ref('')
async function load() {
  error.value = ''
  try { configuration.value = await getClientConfiguration(); emit('changed', { server: configuration.value.id_server, public_key: configuration.value.public_key }) }
  catch (e) { error.value = e instanceof Error ? e.message : 'Unable to load server configuration' }
}
onMounted(load)
</script>
<template>
  <div v-if="configuration?.can_edit" class="flex h-full min-w-0 flex-col items-start justify-between gap-3 rounded-xl border border-gray-200 bg-white p-4 dark:border-rd-border dark:bg-rd-card"><div class="w-full min-w-0"><h2 class="text-sm font-medium">Servidor</h2><p class="mt-1 line-clamp-2 text-xs text-gray-500 dark:text-rd-text-secondary">ID, relay público, API, presença e chave pública em um painel.</p></div><router-link to="/server-settings" class="ui-button !px-3 !py-1.5 !text-xs">Configurar servidor</router-link></div>
  <div class="flex h-full min-w-0 flex-col items-start justify-between gap-3 rounded-xl border border-gray-200 bg-white p-4 dark:border-rd-border dark:bg-rd-card"><div class="w-full min-w-0"><h2 class="text-sm font-medium">Configuração dos clientes</h2><p class="mt-1 truncate text-xs text-gray-500 dark:text-rd-text-secondary">{{ configuration?.id_server || 'Consulte os endereços e a Key para preencher no RustDesk.' }}</p></div><router-link to="/client-setup" class="ui-button !px-3 !py-1.5 !text-xs">Ver e copiar campos</router-link></div>
  <ErrorNotice :message="error" retry @retry="load" />
</template>
