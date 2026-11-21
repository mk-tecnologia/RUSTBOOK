# Instalação e atualização

## Comando único

Execute os comandos deste guia como **root**. Em uma conta comum, entre com
`su -` antes de continuar. Não é necessário ter `sudo` instalado.

Requisitos no servidor: Linux, acesso root, Docker Engine com Compose v2,
Git, curl e Python 3.11 ou superior. O instalador não instala esses requisitos.
Não é necessário instalar Rust ou Node no host: a compilação acontece no Docker.
Reserve espaço para o build, a imagem anterior e uma cópia completa do banco.

```bash
curl -fsSL https://raw.githubusercontent.com/mk-tecnologia/RUSTBOOK/main/scripts/install.py | python3 - install latest
```

Repita o mesmo comando para atualizar. `update latest` também é aceito.
`latest` acompanha a branch `main`, não uma GitHub Release homologada.
O commit efetivamente compilado é registrado no servidor. Faça a homologação
com uma cópia do banco e com seus clientes antes de atualizar a produção.

O instalador usa `/opt/rustbook`:

| Caminho | Conteúdo |
| --- | --- |
| `.env` | Senha inicial, chave JWT e configuração persistente |
| `data/` | SQLite e seus arquivos auxiliares |
| `compose.json` | Serviço da agenda gerenciado pelo instalador |
| `install-state.json` | Versão, commit e imagem instalados |
| `backups/` | Cópias consistentes dos dados e da configuração |

A primeira execução cria credenciais aleatórias e um banco novo. Nas próximas,
a configuração e os usuários são preservados. A senha do `.env` só cria o admin
quando o banco está vazio; ela não redefine senhas de contas existentes.

A imagem é compilada antes da interrupção. Depois, a agenda é parada, os dados
são copiados e a nova imagem é iniciada. A API precisa responder com a versão
esperada em até 60 segundos. Em falha, o instalador tenta restaurar o banco e a
imagem anteriores e retorna erro. Se essa restauração também falhar, mantenha o
serviço parado e use o backup informado na saída. Confirme os logs e o acesso
pelo cliente após cada atualização.

Backups e imagens anteriores não são apagados automaticamente. Monitore o espaço
em disco e mantenha cópias externas: um backup no mesmo disco não protege contra
a perda do servidor. O instalador não configura TLS, firewall, hbbs ou hbbr.

## Acesso e configuração

O endereço padrão é `http://127.0.0.1:21114`. Configure um proxy HTTPS no host
para encaminhar à porta 21114. Proxies em outro container precisam de configuração
de rede própria; o loopback desse container não é o loopback do host.

Consulte as credenciais como root:

```bash
cat /opt/rustbook/.env
```

Para mudar o IP de escuta, edite `BIND_IP` nesse arquivo. Use o IP interno do
servidor para acesso pela LAN ou `0.0.0.0` para todas as interfaces, com firewall
e TLS apropriados. Preserve `RUSTDESK_AB_JWT_SECRET`. Outras opções estão na
[referência de configuração](REFERENCIA.md).

Depois de editar a configuração:

```bash
docker compose -p rustbook --env-file /opt/rustbook/.env -f /opt/rustbook/compose.json up -d
```

Logs:

```bash
docker compose -p rustbook --env-file /opt/rustbook/.env -f /opt/rustbook/compose.json logs --tail=100 address-book
```

Mantenha a porta interna em 21114, o caminho do banco em `/data/db.sqlite3` e os
arquivos gerenciados no mesmo diretório. Edite a configuração no `.env`; o
`compose.json` é regenerado pelo instalador. Variáveis exportadas no seu terminal
não substituem a configuração persistente do `.env` para o container.

## Migrar uma agenda existente

Esta preparação é necessária **uma única vez** para instalações anteriores ao
instalador. Atualizações seguintes usam apenas o comando único.

1. Identifique o diretório real do banco e salve a imagem/configuração anteriores.
   Nas instalações isoladas o banco costuma estar em `data/db.sqlite3`; na stack
   completa, em `data/ab/db.sqlite3`. Não use um diretório vazio por engano.
2. Pare **somente o container da agenda**, mantendo hbbs/hbbr em execução. Faça
   backup consistente conforme [operação](OPERACAO.md). Se ele se chama
   `rustdesk-address-book`, renomeie o container parado para
   `rustdesk-address-book-before-rustbook`, liberando o nome sem removê-lo:

   ```bash
   docker stop rustdesk-address-book
   docker rename rustdesk-address-book rustdesk-address-book-before-rustbook
   ```

3. Prepare uma cópia de migração, por exemplo `/root/rustbook-migration`, contendo
   `data/db.sqlite3` e todos os arquivos WAL/SHM que existirem, além de um `.env`
   com a **chave JWT original** em `RUSTDESK_AB_JWT_SECRET`. Se o Compose antigo
   usa `JWT_SECRET`, copie seu valor para essa variável no arquivo de migração.
   Preserve também as opções de CORS, eventos de dispositivos e IP de escuta
   usadas na instalação. Restrinja o diretório a 0700 e o `.env` a 0600.
4. Com a origem parada, execute a importação inicial:

   ```bash
   curl -fsSL https://raw.githubusercontent.com/mk-tecnologia/RUSTBOOK/main/scripts/install.py | python3 - install latest --import-from /root/rustbook-migration
   ```

O instalador copia a origem para `/opt/rustbook` e preserva os arquivos originais.
Não importa dados em um destino já gerenciado. Ele também recusa a importação
quando detecta um container em execução montando o diretório de origem.
Não reinicie a implantação antiga junto da nova nem use o Compose antigo para
recriar a agenda. Confira proxy, volume, login e leitura/gravação antes de liberar
o acesso. Sessões antigas precisam de novo login na versão 0.2.0.

A importação inicial não conhece a imagem antiga para rollback automático. Se
falhar, pare a nova agenda e retorne ao container/configuração originais, usando
os dados originais preservados. Depois de uma instalação bem-sucedida, as próximas
atualizações passam a ter restauração automática da imagem e do banco anteriores.

## Instalação manual

O `compose.deploy.example.yaml` permanece disponível para quem administra seu
próprio Compose. Veja [implantação isolada](COMPATIBILIDADE-RUSTDESK.md).
O `docker-compose.yml` da raiz inclui hbbs/hbbr e destina-se à stack completa.
Esses métodos têm configuração e procedimento de atualização próprios; o comando
único administra apenas instalações feitas ou importadas por ele.
