<script setup lang="ts">
import { computed, ref } from 'vue'
import type { ClientConfiguration } from '../../api/serverSettings'
import { clientConfigurationText, clientFields, copyText } from '../../utils/clientConfiguration'
const props = defineProps<{ configuration: ClientConfiguration }>()
const message = ref('')
const complete = computed(() => clientFields.every(field => props.configuration[field.key].trim()))
async function copy(value: string) { message.value = await copyText(value) ? 'Copiado.' : 'Selecione o valor e copie manualmente.' }
</script>
<template>
  <section class="rounded-xl border border-gray-200 bg-white p-5 dark:border-rd-border dark:bg-rd-card" aria-label="Configuração dos clientes">
    <div class="mb-4 flex flex-wrap items-start justify-between gap-3"><div><h2 class="font-semibold">Configuração dos clientes</h2><p class="mt-1 text-sm text-gray-500 dark:text-rd-text-secondary">No RustDesk, abra Configurações → Rede → Servidor ID/Relay e preencha estes valores.</p></div><button type="button" :disabled="!complete" class="ui-button" @click="copy(clientConfigurationText(configuration))">Copiar tudo</button></div>
    <p v-if="!complete" class="mb-4 text-sm text-amber-700 dark:text-amber-400">Há campos ainda não configurados pelo administrador.</p>
    <div class="grid gap-4 sm:grid-cols-2">
      <div v-for="field in clientFields" :key="field.key"><label :for="`client-${field.key}`" class="mb-1 block text-xs font-medium">{{ field.label }}</label><div class="flex items-center gap-2"><input :id="`client-${field.key}`" :value="configuration[field.key]" readonly autocomplete="off" spellcheck="false" class="peer-input min-w-0 font-mono text-xs" placeholder="Não configurado"><button type="button" :disabled="!configuration[field.key]" :aria-label="`Copiar ${field.label}`" :title="`Copiar ${field.label}`" class="rounded-lg p-2 text-rd-primary hover:bg-rd-primary/10 disabled:opacity-40" @click="copy(configuration[field.key])"><svg aria-hidden="true" class="size-4" fill="none" stroke="currentColor" stroke-width="1.7" viewBox="0 0 24 24"><path d="M9 9h11v12H9zM5 16H3V3h12v2" /></svg></button></div></div>
    </div>
    <p role="status" class="mt-3 min-h-5 text-xs text-rd-primary">{{ message }}</p>
    <p class="text-xs leading-relaxed text-gray-500 dark:text-rd-text-secondary">Copiar tudo gera um resumo para compartilhar; preencha os campos individualmente no cliente. A Key é a chave pública do servidor. A senha para controlar um computador é a senha do RustDesk daquele computador, solicitada ao conectar.</p>
  </section>
</template>
