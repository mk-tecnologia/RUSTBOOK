# Presença dos dispositivos

Cadastrar um ID na agenda não consulta automaticamente o computador. O indicador
“Pronto” no cliente RustDesk refere-se à conexão dele com seu servidor de ID;
a API da agenda precisa de uma fonte de presença para exibir esse estado.

A partir de 0.5.0, Online e Offline só são exibidos quando o hbbs configurado
responde à consulta nativa de presença. Um relatório recente da API indica
“Recent activity”; relatórios antigos ou ausentes indicam “Unknown”, sem
concluir que o equipamento está offline. O tooltip explica a origem do estado.
A lista aberta atualiza a cada 30 segundos e ao voltar à janela.

## Configurar no servidor Linux

Execute como root, substituindo `SEU_SERVIDOR_ID` pelo hostname/IP do mesmo hbbs
configurado nos clientes RustDesk:

```bash
curl -fsSL https://raw.githubusercontent.com/mk-tecnologia/RUSTBOOK/main/scripts/install.py | python3 - install latest --hbbs SEU_SERVIDOR_ID:21115
```

A porta é a NAT TCP do hbbs: normalmente 21115, uma abaixo da porta do servidor
de ID (normalmente 21116). Não use a porta 21117 do relay. O endereço precisa
ser alcançável pelo container da agenda. Não use `localhost`/`127.0.0.1`: além
de apontar para o próprio container, o hbbs trata conexões de loopback nessa
porta como console administrativo, em vez do protocolo de presença.

Se hbbs e agenda compartilham uma rede Docker, use o nome do serviço, como
`--hbbs hbbs:21115`. O `docker-compose.yml` completo já usa esse endereço.
Não é necessário habilitar eventos anônimos nem alterar a autenticação.

O instalador guarda essa configuração para as próximas atualizações. Depois,
use o comando habitual sem `--hbbs`. Para desativar a consulta, execute com
`--hbbs ''`. Em instalações manuais, configure a variável
`RUSTDESK_AB_RENDEZVOUS_SERVER=hostname:21115` e recrie apenas o serviço da agenda,
ou use `rendezvous_server` no arquivo de configuração.

## Se continuar desconhecido ou offline

- “Unknown” com erro de consulta: verifique endereço, rede e acesso TCP à porta
  NAT a partir do container da agenda. O timeout é de três segundos; falhas não
  são convertidas em Offline.
- “Offline” confirmado: verifique se o cliente usa exatamente o mesmo servidor
  de ID consultado pela agenda e se o ID cadastrado está correto.
- A presença continua restrita aos administradores, como os dados de inventário
  anteriores. Compartilhar uma agenda não concede acesso ao inventário global.

A consulta usa OnlineRequest/OnlineResponse do protocolo RustDesk; ela não abre
uma sessão remota nem tenta conectar às portas do dispositivo.
