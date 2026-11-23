# HTTPS para a agenda e API

O campo Servidor da API é o endereço informado aos clientes. Salvar `https://...`
não instala certificado nem ativa TLS. A agenda pode continuar usando HTTP em
loopback/rede privada enquanto um proxy atende HTTPS externamente.

## Antes de implantar

- O domínio deve apontar para o IP público correto (A e também AAAA, se existir).
- Verifique `ss -lntp '( sport = :80 or sport = :443 )'` e `docker ps`.
  Se já houver proxy, configure o domínio nele em vez de iniciar outro nas mesmas portas.
- Encaminhe TCP 80 e 443 do IP público ao host do proxy e libere o firewall.
  Portas livres no host não comprovam encaminhamento no roteador.
- Preserve os serviços hbbs/hbbr, os volumes e as chaves existentes.

## Modelo Docker opcional com Caddy

O modelo em [deploy/https](../deploy/https/compose.yaml) usa a rede do host Linux;
assim o proxy alcança a porta da agenda publicada no host. O container não recebe
socket Docker nem volumes da agenda/hbbs. Os certificados têm volume próprio persistente.
O Caddy emite/renova certificados e redireciona HTTP para HTTPS automaticamente,
desde que domínio, portas e validação pública estejam corretos.

Como root, para uma implantação nova do proxy, baixe os dois arquivos para
`/opt/rustbook-https`. Não substitua arquivos existentes sem revisá-los:

```bash
mkdir -p /opt/rustbook-https
cd /opt/rustbook-https
curl -fSLo compose.yaml https://raw.githubusercontent.com/mk-tecnologia/RUSTBOOK/main/deploy/https/compose.yaml
curl -fSLo Caddyfile https://raw.githubusercontent.com/mk-tecnologia/RUSTBOOK/main/deploy/https/Caddyfile
```

Crie `.env` nesse diretório com **seu domínio e o endpoint HTTP que funciona hoje**:

```dotenv
RUSTBOOK_HTTPS_DOMAIN=agenda.example.com
RUSTBOOK_HTTPS_UPSTREAM=127.0.0.1:21114
```

Se a agenda estiver publicada somente no IP LAN, use esse IP no upstream primeiro.
Não use `127.0.0.1` enquanto a agenda não estiver escutando em loopback.
Confirme o endpoint com `curl -fsS http://ENDERECO:21114/api/version`.

```bash
docker compose --project-name rustbook-https config --quiet
docker compose --project-name rustbook-https up -d
docker compose --project-name rustbook-https logs --tail=80 https
```

## Validar antes de mudar os clientes

Substitua o domínio no teste e execute também a partir de outra rede:

```bash
curl -fsS https://agenda.example.com/api/version
```

Não use `-k`: o teste precisa validar a confiança e o nome do certificado.
Se falhar, revise DNS, encaminhamento 80/443, firewall e logs do proxy.
Não troque a API dos clientes enquanto HTTPS não funcionar.

Depois de validar, salve `https://agenda.example.com` em **Server settings →
Servidor da API** e copie esse valor para a API dos clientes RustDesk.
O painel fica no mesmo endereço HTTPS; não acrescente `/api` nem `:21114` à URL.
Os endereços de ID, relay e presença e suas portas 21115–21117 permanecem iguais.

## Impedir acesso direto por HTTP

1. Remova o encaminhamento público da porta 21114 no roteador/firewall.
2. Para bloquear também acesso HTTP direto pela LAN, publique a agenda somente
   em `127.0.0.1:21114:21114` no Compose que define suas portas.
   **Em uma instalação adotada, `BIND_IP` no `.env` pode não controlar a publicação:**
   confira os arquivos mostrados por `docker inspect rustdesk-address-book --format
   '{{index .Config.Labels "com.docker.compose.project.config_files"}}'`.
   Preserve o banco, JWT, senhas e volumes. Faça backup antes de editar o Compose.
3. Recrie somente o serviço da agenda com seu launcher `/opt/rustbook/manage.sh
   up -d --no-deps address-book` (ajuste o nome do serviço caso seja outro).
4. Mude `RUSTBOOK_HTTPS_UPSTREAM=127.0.0.1:21114` no `.env` do proxy e execute
   `docker compose --project-name rustbook-https up -d` em `/opt/rustbook-https`.
5. Teste novamente o HTTPS e confirme que o IP LAN/público não serve mais a agenda
   diretamente na porta 21114. Docker publica portas com regras próprias; não
   considere apenas uma regra genérica do firewall como prova do bloqueio.

A porta 80 pode permanecer atendendo apenas ao redirecionamento e à validação ACME;
ela não deve servir a agenda/API em HTTP. Se a exigência for fechar totalmente a
porta 80, planeje o método de emissão/renovação antes de bloqueá-la (por exemplo,
validação TLS-ALPN em 443). Redirecionar HTTP não protege uma senha já enviada por
um cliente antigo: atualize a URL dos clientes e feche o acesso direto à 21114.

Referências: [HTTPS automático](https://caddyserver.com/docs/automatic-https),
[proxy reverso](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy),
[Caddy com Docker Compose](https://caddyserver.com/docs/running#docker-compose).
