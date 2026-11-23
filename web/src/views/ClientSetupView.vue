<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { getClientConfiguration, type ClientConfiguration } from '../api/serverSettings'
import ClientConfigurationPanel from '../components/server/ClientConfigurationPanel.vue'
import ErrorNotice from '../components/ui/ErrorNotice.vue'
const configuration = ref<ClientConfiguration>()
const error = ref('')
async function load() { error.value = ''; try { configuration.value = await getClientConfiguration() } catch (e) { error.value = e instanceof Error ? e.message : 'Unable to load client configuration' } }
onMounted(load)
</script>
<template><div class="space-y-5"><div class="flex flex-wrap items-center justify-between gap-3"><h1 class="text-xl font-semibold">Configuração dos clientes</h1><router-link v-if="configuration?.can_edit" to="/server-settings" class="ui-button">Configurar servidor</router-link></div><ErrorNotice :message="error" retry @retry="load" /><ClientConfigurationPanel v-if="configuration" :configuration="configuration" /><p v-else-if="!error" role="status" class="text-sm text-gray-500">Carregando…</p></div></template>
