# Painel de configuração do servidor (0.7.0+)

Administradores acessam **Server settings** no menu. O painel reúne os endereços
públicos de ID, relay e API, a Key pública e o endpoint interno de presença.
Configurações anteriores de presença e conexão são preservadas e exibidas no painel.

## Campos de rede

| Campo | Finalidade | Porta usual |
| --- | --- | --- |
| Servidor de ID para os clientes | Domínio/IP acessível pelos clientes RustDesk | TCP/UDP 21116 |
| Relay público anunciado pelo hbbs | Endereço do hbbr entregue aos clientes pelo argumento `hbbs -r` | TCP 21117 |
| Servidor da API para os clientes | Origem HTTP(S) pública da agenda, sem `/api` no final | HTTPS 443 ou HTTP 21114 |
| Consulta de presença | Endpoint alcançável pelo container da agenda; pode ser interno | TCP 21115 |
| Key pública | Chave já existente do hbbs, também configurada nos clientes | Não é senha nem chave privada |

O endereço público do relay deve funcionar fora da LAN. O painel avisa sobre
endereços privados/locais; a aplicação desses endereços exige confirmação de uso
somente em rede interna/VPN. Isso não substitui testes a partir da rede externa.
Use HTTPS para a API exposta à Internet.

**Salvar configuração** atualiza a agenda, a consulta de presença e os valores
copiados aos clientes. **Aplicar relay ao hbbs** é uma ação separada. O painel
mostra o valor salvo, o argumento do container em execução e o valor persistido
no Compose. Não informa aplicação bem-sucedida apenas por salvar um campo.

Portas publicadas, NAT, firewall, certificados TLS, credenciais administrativas,
JWT, origens CORS e expiração de sessão permanecem na implantação do host. Os
parâmetros ativos da agenda são exibidos em leitura; o painel não altera portas
ou segredos sem um procedimento específico de implantação.

## Habilitar aplicação pelo painel

O container web não tem acesso ao socket Docker. Um agente restrito no host
Linux recebe somente duas operações: consultar o hbbs e alterar seu relay anunciado.

Como root no host Docker com systemd, execute uma vez, ajustando o caminho do
Compose **existente** que já executa seu hbbs:

```bash
curl -fsSL https://raw.githubusercontent.com/mk-tecnologia/RUSTBOOK/main/scripts/install.py | python3 - install latest --manage-hbbs /opt/rustdesk/compose.yml
```

Se o serviço não se chama `hbbs`, acrescente `--hbbs-service NOME_DO_SERVICO`.
O instalador identifica o projeto e os arquivos pelos rótulos do container
existente; não cria outra stack hbbs/hbbr. A instalação do agente **não reinicia**
esses serviços. Atualizações futuras da agenda mantêm e atualizam o agente.

Pré-requisitos: Docker rootful/Compose, serviço hbbs em execução, comando explícito
no Compose e par `id_ed25519`/`id_ed25519.pub` persistido em volume no diretório
de trabalho do hbbs. Se o agente não consegue verificar a implantação ou as chaves,
a aplicação é recusada sem trocar o serviço. Compose com configurações indiretas
que não permitem localizar o comando pode exigir revisão manual.

Depois de atualizar, recarregue o painel:

1. Preencha os endereços de ID, relay e API e mantenha a Key pública existente.
2. Se necessário, use **Usar relay atual** e **Usar chave existente do hbbs**.
3. Salve e confira os avisos e a seção **Configuração dos clientes**.
4. Teste a conectividade salva.
5. Confira o serviço/Compose selecionados e confirme **Aplicar relay ao hbbs**.
6. Faça uma conexão entre clientes externos que realmente utilize o relay.

Uma porta TCP acessível não comprova autenticação no hbbr. Mensagens `invalid key`
precisam ser associadas ao mesmo teste e à configuração efetiva do relay.

## Preservação e rollback

O agente altera somente o `command` do serviço hbbs no Compose existente, mantendo
os demais argumentos, inclusive `-k`. O formato do comando pode ser normalizado
para uma lista JSON válida em YAML. O hbbr e o serviço da agenda não são recriados.

Antes de aplicar, são verificados os volumes e o par de chaves persistido. O agente
cria backup privado do comando e das chaves em `/opt/rustbook/server-backups`,
preserva a imagem do container existente e usa `--no-deps --no-build --pull never`.
O argumento do relay fica persistido para futuras recriações pelo Compose normal.
Não são geradas novas chaves. Se a aplicação falha, o agente tenta restaurar o
comando e as chaves anteriores e confirma o processo em execução; falha de rollback
é indicada como erro que exige revisão do host, sem afirmar que o serviço voltou.

Reaplicar um relay já ativo e persistido não reinicia hbbs. Se apenas o container
já estava correto, o Compose é corrigido sem reiniciar o processo.

O agente usa `.rustbook-server-agent.sock` dentro do volume da agenda, com acesso
restrito ao UID 10001. O instalador exclui esse socket dos backups e o preserva
num rollback dos dados. Nenhuma operação de shell, caminho ou serviço arbitrário
é aceita pelo protocolo. Alterações não podem ser aplicadas durante uma atualização
da agenda. Os arquivos do agente e backups de chave no host são exclusivos de root.

O histórico administrativo registra salvamento e solicitação/conclusão de aplicação.
Consulte o agente com:

```bash
systemctl status rustbook-server-agent.service
journalctl -u rustbook-server-agent.service --since '10 minutes ago'
```

## Campos prontos para os clientes

A página **Client setup** é acessível a usuários autenticados e mostra exatamente:

- Servidor de ID;
- Servidor Relay;
- Servidor da API;
- Chave / Key pública.

Há botões para copiar cada campo e **Copiar tudo** para um resumo de configuração.
O resumo é para compartilhamento; não é um código de importação do RustDesk.
Campos internos, token da conta, JWT e chave privada nunca são exportados.
A cópia também dispõe de alternativa para navegadores em HTTP de rede local.

A senha remota continua sendo a senha do RustDesk do computador de destino.
Para a limitação de conta nativa com hbbs OSS, veja [compatibilidade](COMPATIBILIDADE-RUSTDESK.md).
