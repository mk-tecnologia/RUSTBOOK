import type { ServerHealth } from '../api/serverSettings'

export type HealthState = 'ok' | 'warning' | 'error' | 'missing'
export interface HealthItem { key: string; label: string; state: HealthState; status: string; detail: string }
export function serverHealthItems({ report, checks }: ServerHealth): HealthItem[] {
  const { settings, agent } = report
  const items: HealthItem[] = [{ key: 'application', label: 'Agenda / API atual', state: 'ok', status: 'Respondendo', detail: 'A agenda respondeu à verificação autenticada.' }]
  for (const [key, label] of [['id_server', 'Servidor de ID'], ['relay_server', 'Servidor Relay'], ['api_url', 'API dos clientes'], ['presence_server', 'Presença dos dispositivos']] as const) {
    const endpoint = settings[key]
    const check = checks[key]
    const state: HealthState = !endpoint ? 'missing' : check === 'tcp_reachable' || check === 'protocol_replied' ? 'ok' : check === 'unreachable' ? 'error' : 'warning'
    items.push({ key, label, state, status: state === 'missing' ? 'Falta configurar' : check === 'protocol_replied' ? 'Protocolo respondeu' : check === 'tcp_reachable' ? 'TCP acessível' : state === 'error' ? 'Sem resposta' : 'Sem confirmação', detail: !endpoint ? (key === 'presence_server' ? 'Informe o endpoint hbbs, normalmente na porta 21115, para consultar online/offline.' : 'Preencha este endereço em Configuração do servidor.') : endpoint })
  }
  const keyMissing = !settings.public_key
  const keyMismatch = Boolean(agent.public_key && settings.public_key && agent.public_key !== settings.public_key)
  items.push({ key: 'key', label: 'Chave pública / Key', state: keyMissing ? 'missing' : keyMismatch ? 'error' : agent.public_key ? 'ok' : 'warning', status: keyMissing ? 'Falta configurar' : keyMismatch ? 'Chave diferente' : agent.public_key ? 'Confere com o hbbs' : 'Salva, sem comparação', detail: keyMissing ? 'Use a chave pública existente do servidor.' : keyMismatch ? 'A chave salva difere da chave do hbbs. Corrija antes de configurar os clientes.' : agent.public_key ? 'A chave salva corresponde à chave pública do hbbs gerenciado.' : 'Ative o agente para comparar com a chave existente no hbbs.' })
  items.push({ key: 'agent', label: 'Agente e serviço hbbs', state: !agent.connected ? 'warning' : !agent.running ? 'error' : !agent.can_apply ? 'warning' : 'ok', status: !agent.connected ? 'Agente não conectado' : !agent.running ? 'hbbs parado ou não confirmado' : !agent.can_apply ? 'Gerenciamento indisponível' : 'Conectado', detail: !agent.connected ? 'Ative o agente no host para confirmar o hbbs e aplicar o relay pelo painel.' : agent.error || (agent.running ? 'O agente confirmou o hbbs em execução.' : 'Verifique o serviço hbbs no host.') })
  const relayMatches = Boolean(settings.relay_server && agent.relay === settings.relay_server && agent.configured_relay === settings.relay_server)
  items.push({ key: 'relay_applied', label: 'Relay anunciado pelo hbbs', state: !settings.relay_server ? 'missing' : !agent.connected || !agent.running || !agent.relay || !agent.configured_relay ? 'warning' : relayMatches ? 'ok' : 'warning', status: !settings.relay_server ? 'Falta configurar' : !agent.connected || !agent.running || !agent.relay || !agent.configured_relay ? 'Sem confirmação' : relayMatches ? 'Ativo e persistido' : 'Aplicação pendente', detail: relayMatches && agent.connected && agent.running ? 'O relay em execução e o Compose correspondem ao valor salvo.' : !settings.relay_server ? 'Defina o endereço público do relay.' : !agent.connected || !agent.running ? 'É necessário confirmar o serviço pelo agente.' : `Salvo: ${settings.relay_server} · Em execução: ${agent.relay || 'não confirmado'} · Compose: ${agent.configured_relay || 'não confirmado'}` })
  return items
}
export function serverHealthSummary(items: HealthItem[], warnings: string[]) {
  const errors = items.filter(item => item.state === 'error').length
  const missing = items.filter(item => item.state === 'missing').length
  const pending = items.filter(item => item.state === 'warning').length
  return { state: errors ? 'error' : missing || pending || warnings.length ? 'warning' : 'ok', label: errors ? 'Há falhas nas verificações' : missing ? 'Configuração incompleta' : pending || warnings.length ? 'Há pontos para revisar' : 'Verificações sem pendências', errors, missing, pending }
}
