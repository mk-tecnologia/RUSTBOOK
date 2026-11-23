<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { applyRelay, checkServerSettings, getServerSettings, saveServerSettings, type ServerReport, type ServerSettings } from '../api/serverSettings'
import ClientConfigurationPanel from '../components/server/ClientConfigurationPanel.vue'
import ErrorNotice from '../components/ui/ErrorNotice.vue'
import BaseDialog from '../components/ui/BaseDialog.vue'
const report = ref<ServerReport>()
const form = ref<ServerSettings>()
const busy = ref(false)
const error = ref('')
const notice = ref('')
const diagnostics = ref<Record<string,string>>()
const confirming = ref(false)
const route = useRoute()
async function focusSection() {
  await nextTick()
  const target = document.getElementById(route.hash.slice(1))
  if (!target || !route.hash) return
  target.scrollIntoView({ block: 'center', behavior: 'smooth' })
  target.focus({ preventScroll: true })
}
watch(() => route.hash, focusSection)
function editApi() { document.getElementById('server-api')?.focus() }
const dirty = computed(() => JSON.stringify(form.value) !== JSON.stringify(report.value?.settings))
const pending = computed(() => Boolean(report.value?.settings.relay_server && (report.value.settings.relay_server !== report.value.agent.relay || report.value.settings.relay_server !== report.value.agent.configured_relay)))
async function load() { error.value = ''; try { report.value = await getServerSettings(); form.value = { ...report.value.settings }; await focusSection() } catch (e) { error.value = e instanceof Error ? e.message : 'Unable to load settings' } }
async function save() {
  if (!form.value || busy.value) return
  busy.value = true; error.value = ''; notice.value = ''
  try { report.value = await saveServerSettings(form.value); form.value = { ...report.value.settings }; diagnostics.value = undefined; notice.value = 'Configuração salva. O relay anunciado pelo hbbs só muda após Aplicar relay.' }
  catch (e) { error.value = e instanceof Error ? e.message : 'Unable to save settings' }
  finally { busy.value = false }
}
async function test() {
  busy.value = true; error.value = ''
  try { diagnostics.value = await checkServerSettings() } catch (e) { error.value = e instanceof Error ? e.message : 'Unable to test connectivity' }
  finally { busy.value = false }
}
async function apply() {
  if (!report.value || busy.value) return
  busy.value = true; error.value = ''; notice.value = ''
  try { const result = await applyRelay(report.value.settings.revision); confirming.value = false; await load(); notice.value = result.changed ? (result.restarted ? 'Relay aplicado ao hbbs. Chave e imagem existentes preservadas. Valide uma conexão entre clientes externos.' : 'O relay já estava ativo. A configuração persistente foi corrigida sem reiniciar o hbbs.') : 'O hbbs e o Compose já anunciam esse relay; nenhum reinício necessário.' }
  catch (e) { const failure = e instanceof Error ? e.message : 'Unable to apply relay'; await load(); error.value = failure; confirming.value = false }
  finally { busy.value = false }
}
function useExistingKey() { if (form.value && report.value?.agent.public_key) form.value.public_key = report.value.agent.public_key }
const labels: Record<string,string> = { id_server: 'Servidor de ID (TCP)', relay_server: 'Relay (TCP)', presence_server: 'Presença (protocolo hbbs)' }
const results: Record<string,string> = { tcp_reachable: 'Porta TCP acessível', protocol_replied: 'Protocolo respondeu', not_configured: 'Não configurado', unreachable: 'Não respondeu' }
onMounted(load)
</script>
<template>
  <div class="mx-auto max-w-6xl space-y-6">
    <header><h1 class="text-xl font-semibold">Configuração do servidor</h1><p class="mt-1 text-sm text-gray-500 dark:text-rd-text-secondary">Endereços dos clientes, presença e relay anunciado pelo hbbs, em um só lugar.</p></header>
    <ErrorNotice :message="error" retry @retry="load" /><p v-if="notice" role="status" class="rounded-xl border border-blue-200 bg-blue-50 p-4 text-sm text-blue-800 dark:border-blue-900 dark:bg-blue-950/20 dark:text-blue-300">{{ notice }}</p>
    <p v-if="!form && !error" role="status" class="text-sm text-gray-500">Carregando configurações…</p>
    <template v-if="form && report">
      <form class="rounded-xl border border-gray-200 bg-white p-5 dark:border-rd-border dark:bg-rd-card" @submit.prevent="save">
        <h2 class="mb-4 font-semibold">Endereços e chave pública</h2>
        <fieldset :disabled="busy" class="grid gap-4 md:grid-cols-2">
          <div><label for="server-id" class="mb-1 block text-xs font-medium">Servidor de ID para os clientes</label><input id="server-id" v-model="form.id_server" maxlength="255" class="peer-input" placeholder="rustdesk.example.com:21116"><p class="mt-1 text-xs text-gray-500">Endereço acessível pelos computadores locais e externos. Porta normalmente 21116.</p></div>
          <div><label for="server-relay" class="mb-1 block text-xs font-medium">Relay público anunciado pelo hbbs</label><div class="flex flex-wrap gap-2"><input id="server-relay" v-model="form.relay_server" maxlength="255" class="peer-input min-w-0 flex-1" placeholder="rustdesk.example.com:21117"><button v-if="report.agent.relay" type="button" class="ui-button" @click="form.relay_server = report.agent.relay || ''">Usar relay atual</button></div><p class="mt-1 text-xs text-gray-500">Use domínio ou IP alcançável por todos os clientes. Porta normalmente 21117.</p></div>
          <div><label for="server-api" class="mb-1 block text-xs font-medium">Servidor da API para os clientes</label><input id="server-api" v-model="form.api_url" type="url" maxlength="512" class="peer-input" placeholder="https://rustdesk.example.com"><p class="mt-1 text-xs text-gray-500">URL pública da agenda, sem caminho /api. Em HTTP direto, normalmente porta 21114.</p></div>
          <div><label for="server-presence" class="mb-1 block text-xs font-medium">Consulta de presença a partir do RUSTBOOK</label><input id="server-presence" v-model="form.presence_server" maxlength="255" class="peer-input" placeholder="hbbs:21115"><p class="mt-1 text-xs text-gray-500">Pode ser interno/Docker. Porta NAT TCP normalmente 21115; não é o endereço copiado aos clientes.</p></div>
          <div class="md:col-span-2"><label for="server-key" class="mb-1 block text-xs font-medium">Key pública existente do servidor</label><div class="flex flex-wrap gap-2"><input id="server-key" v-model="form.public_key" maxlength="44" autocomplete="off" spellcheck="false" class="peer-input min-w-0 flex-1 font-mono" placeholder="Key pública configurada nos clientes"><button v-if="report.agent.public_key" type="button" class="ui-button" @click="useExistingKey">Usar chave existente do hbbs</button></div><p class="mt-1 text-xs text-gray-500">Nenhuma chave é gerada ou rotacionada pelo painel. Nunca cole a chave privada.</p></div>
          <label class="flex items-start gap-2 text-sm md:col-span-2"><input v-model="form.allow_private_relay" type="checkbox" class="mt-1 accent-rd-primary"><span>Uso somente em rede interna/VPN: permitir aplicar um relay com endereço privado.</span></label>
        </fieldset>
        <div class="mt-5 flex flex-wrap justify-end gap-2 border-t border-gray-100 pt-4 dark:border-rd-border"><button type="button" :disabled="busy" class="ui-button" @click="load">Recarregar</button><button type="submit" :disabled="busy || !dirty" class="rounded-lg bg-rd-primary px-4 py-2 text-sm font-medium text-white disabled:opacity-50">{{ busy ? 'Aguarde…' : 'Salvar configuração' }}</button></div>
      </form>
      <ul v-if="report.warnings.length" class="space-y-1 rounded-xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-800 dark:border-amber-900 dark:bg-amber-950/20 dark:text-amber-300"><li v-for="warning in report.warnings" :key="warning">{{ warning }}</li></ul>
      <section id="server-agent" tabindex="-1" class="scroll-mt-6 rounded-xl border border-gray-200 bg-white p-5 dark:border-rd-border dark:bg-rd-card">
        <div class="flex flex-wrap items-start justify-between gap-3"><div><h2 class="font-semibold">Aplicação no hbbs</h2><p class="mt-1 text-sm text-gray-500 dark:text-rd-text-secondary">Salvar a agenda e aplicar o relay são etapas separadas.</p></div><span class="rounded-full px-3 py-1 text-xs" :class="report.agent.connected && report.agent.running && report.settings.relay_server && !pending ? 'bg-green-100 text-green-700 dark:bg-green-900/20 dark:text-green-300' : 'bg-amber-100 text-amber-800 dark:bg-amber-900/20 dark:text-amber-300'">{{ !report.agent.connected ? 'Agente não conectado' : !report.agent.running ? 'hbbs não está em execução' : !report.settings.relay_server ? 'Relay não configurado' : pending ? 'Relay pendente de aplicação' : 'Relay corresponde ao salvo' }}</span></div>
        <dl class="mt-4 grid gap-3 text-sm sm:grid-cols-2"><div><dt class="text-xs text-gray-500">Relay salvo</dt><dd class="mt-1 break-all font-mono">{{ report.settings.relay_server || 'Não configurado' }}</dd></div><div><dt class="text-xs text-gray-500">Relay anunciado pelo hbbs</dt><dd class="mt-1 break-all font-mono">{{ report.agent.relay || 'Sem confirmação do agente' }}</dd></div><div><dt class="text-xs text-gray-500">Relay persistido no Compose</dt><dd class="mt-1 break-all font-mono">{{ report.agent.configured_relay || 'Sem confirmação do agente' }}</dd></div></dl>
        <div v-if="!report.agent.connected" class="mt-4 rounded-lg bg-gray-50 p-3 text-xs dark:bg-rd-scaffold"><p>Ative o agente uma vez no host Docker. Ajuste o caminho do Compose existente:</p><code class="mt-2 block overflow-x-auto whitespace-pre-wrap break-all">curl -fsSL https://raw.githubusercontent.com/mk-tecnologia/RUSTBOOK/main/scripts/install.py | python3 - install latest --manage-hbbs /CAMINHO/DO/compose.yml</code><p class="mt-2 text-gray-500">A instalação do agente não reinicia hbbs/hbbr. Depois, recarregue este painel.</p></div>
        <p v-else-if="!report.agent.can_apply" class="mt-3 text-sm text-amber-700 dark:text-amber-400">{{ report.agent.error || 'O agente não confirmou o serviço em execução e o volume da chave. A aplicação está bloqueada.' }}</p>
        <div id="server-apply-relay" tabindex="-1" class="mt-4 flex scroll-mt-6 flex-wrap gap-2"><button type="button" :disabled="busy || dirty || !report.agent.can_apply || !report.settings.relay_server" class="ui-button" @click="confirming = true">Aplicar relay ao hbbs</button><button type="button" :disabled="busy || dirty" class="ui-button" @click="test">Testar conectividade salva</button></div>
        <p v-if="report.agent.compose_file" class="mt-3 break-all text-xs text-gray-500">Serviço: {{ report.agent.service }} · Compose: {{ report.agent.compose_file }}</p>
        <p v-if="dirty" class="mt-2 text-xs text-amber-700 dark:text-amber-400">Salve ou recarregue suas alterações antes de testar/aplicar.</p>
        <div v-if="diagnostics" class="mt-4"><dl class="grid gap-3 sm:grid-cols-3"><div v-for="key in ['id_server','relay_server','presence_server']" :key="key"><dt class="text-xs text-gray-500">{{ labels[key] }}</dt><dd class="mt-1 text-sm">{{ results[diagnostics[key] || ''] || diagnostics[key] }}</dd></div></dl><p class="mt-3 text-xs text-gray-500">Porta TCP acessível não comprova autenticação do relay. Valide uma conexão entre clientes externos.</p></div>
      </section>
      <section id="server-https" tabindex="-1" class="scroll-mt-6 rounded-xl border border-gray-200 bg-white p-5 dark:border-rd-border dark:bg-rd-card">
        <h2 class="font-semibold">HTTPS para a agenda e API</h2>
        <p class="mt-2 text-sm text-gray-500 dark:text-rd-text-secondary">O campo Servidor da API informa o endereço aos clientes; ele não instala certificados nem ativa TLS no host.</p>
        <ol class="mt-4 list-inside list-decimal space-y-2 text-sm"><li>Configure um proxy HTTPS no host (ou no proxy já existente), com certificado válido para seu domínio e encaminhamento à agenda na porta 21114.</li><li>Libere/encaminhe a porta 443 para esse proxy. Se o certificado usar validação HTTP, encaminhe também a porta 80.</li><li>Teste https://SEU-DOMINIO/api/version sem ignorar erros de certificado.</li><li>Só depois salve https://SEU-DOMINIO no campo Servidor da API e atualize os clientes RustDesk.</li><li>Restrinja a porta 21114 ao proxy e remova seu acesso externo direto. Configure a porta 80 para redirecionar ao HTTPS ou recusá-la, conforme a emissão/renovação do certificado.</li></ol>
        <p class="mt-3 text-xs text-gray-500 dark:text-rd-text-secondary">O proxy pode usar HTTP internamente na máquina. Para um proxy instalado no mesmo host, publique a agenda em 127.0.0.1:21114; um proxy em outro container precisa de uma rede Docker compartilhada. As portas de ID/relay 21115–21117 continuam com seus protocolos próprios.</p>
        <a href="https://github.com/mk-tecnologia/RUSTBOOK/blob/main/docs/HTTPS.md" target="_blank" rel="noopener noreferrer" class="mt-3 inline-block text-sm font-medium text-rd-primary hover:underline">Abrir guia de implantação HTTPS</a>
        <button type="button" class="ui-button mt-4" @click="editApi">Editar endereço da API</button>
      </section>
      <ClientConfigurationPanel :configuration="{ ...report.settings, can_edit: true }" />
      <details class="rounded-xl border border-gray-200 bg-white p-5 dark:border-rd-border dark:bg-rd-card"><summary class="cursor-pointer text-sm font-medium">Parâmetros ativos da agenda (somente leitura)</summary><dl class="mt-4 grid gap-4 text-sm sm:grid-cols-2"><div><dt>Porta HTTP interna</dt><dd>{{ report.runtime.port }}</dd></div><div><dt>Expiração de sessão</dt><dd>{{ report.runtime.token_expiry_hours }} horas</dd></div><div><dt>Origens CORS</dt><dd class="break-all">{{ report.runtime.cors_origins.join(', ') || 'Mesma origem' }}</dd></div><div><dt>Eventos de dispositivos sem login</dt><dd>{{ report.runtime.anonymous_device_events ? 'Habilitados' : 'Desabilitados' }}</dd></div></dl><p class="mt-3 text-xs text-gray-500">Portas publicadas, firewall, TLS, credenciais administrativas e políticas da agenda continuam na implantação do host.</p></details>
    </template>
  </div>
  <BaseDialog v-if="confirming && report" title="Aplicar relay ao hbbs" :busy="busy" @close="confirming = false"><p class="text-sm">O agente fará backup e recriará somente o serviço hbbs, preservando a imagem, os volumes e a chave existente. O hbbr e a agenda não serão reiniciados.</p><p class="my-4 break-all font-mono text-sm">{{ report.agent.relay || '(atual desconhecido)' }} → {{ report.settings.relay_server }}</p><p class="text-xs text-gray-500">Novas conexões podem precisar aguardar a reinicialização do hbbs.</p><div class="mt-5 flex justify-end gap-2"><button type="button" :disabled="busy" class="ui-button" @click="confirming = false">Cancelar</button><button type="button" :disabled="busy" class="rounded-lg bg-rd-primary px-4 py-2 text-sm text-white disabled:opacity-50" @click="apply">{{ busy ? 'Aplicando…' : 'Confirmar aplicação' }}</button></div></BaseDialog>
</template>
