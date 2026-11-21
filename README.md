# RustBook — RustDesk Address Book

Servidor de agenda e console web para RustDesk OSS. Este projeto é um **clone
com alterações próprias** do repositório
[ds4a/rustdesk-address-book](https://github.com/ds4a/rustdesk-address-book),
a partir do commit [`ca39fa3`](https://github.com/ds4a/rustdesk-address-book/commit/ca39fa3).
É mantido de forma independente por [mk-tecnologia](https://github.com/mk-tecnologia)
no repositório público [mk-tecnologia/RUSTBOOK](https://github.com/mk-tecnologia/RUSTBOOK),
para versionar as alterações e permitir a atualização das instalações próprias.
Os créditos pelo código original pertencem aos autores do upstream.
Backend Rust/Axum, interface Vue 3 + TypeScript e banco SQLite. O frontend é
compilado dentro do binário; o servidor de agenda usa a porta **21114**.

Versão de desenvolvimento: **0.2.0**. A publicação do código não equivale a uma
release homologada para produção. A compatibilidade precisa ser homologada
com as versões dos clientes usadas na sua instalação.

## Funcionalidades

- Login com Argon2 e sessões JWT persistidas e revogáveis.
- Agenda pessoal, leitura de agendas compartilhadas e permissões por usuário/grupo.
- CRUD de dispositivos e tags, paginação, pesquisa e filtro no servidor.
- Gerenciamento administrativo de usuários e grupos.
- Recepção de eventos de dispositivos e consulta paginada de auditoria para admins.
- Console com temas claro/escuro/automático, navegação responsiva e estados de erro.
- Versão do servidor em execução exibida no cabeçalho.
- Migrações SQL embutidas, transacionais e com detecção de alteração de fonte.

A aba Dispositivos do cliente tem consultas básicas de inventário para admins;
usuários comuns recebem lista vazia enquanto não houver ACLs de dispositivos.

O console ainda não oferece criação de compartilhamentos nem edição de membros
nos grupos. Essas relações existem no banco, mas não devem ser anunciadas como
funcionalidades completas da interface. `hbbs` e `hbbr` continuam responsáveis por
registro, descoberta e relay das conexões; este serviço não os substitui.

## Desenvolvimento local

Requisitos: Rust stable com rustfmt/Clippy, Node.js 22+, npm e Python 3.11+.
A validação desta continuação utilizou Rust 1.100.0. Não há MSRV anterior homologada.

```bash
npm --prefix web ci
npm --prefix web run build
export RUSTDESK_AB_JWT_SECRET="$(openssl rand -hex 32)"
export RUSTDESK_AB_ADMIN_PASSWORD='replace-with-a-long-unique-password'
cargo run --locked
```

Abra `http://localhost:21114`. O administrador é criado apenas quando não existem
usuários. A senha inicial deve ter ao menos 12 bytes e não possui valor padrão.
Não há rota pública de redefinição: mantenha acesso administrativo e backup.

Para hot reload, mantenha o backend rodando e execute em outro terminal:

```bash
npm --prefix web run dev
```

O Vite encaminha `/api` para `localhost:21114`. Recompile a interface e o backend
quando quiser atualizar o frontend embutido do binário.

## Docker Compose

O Compose incluído executa `hbbs`, `hbbr` e o build local deste serviço. Antes de
subir, crie `.env` com `JWT_SECRET` (32 bytes ou mais) e `ADMIN_PASSWORD` (12 bytes
ou mais). Gere uma chave com `openssl rand -hex 32` e use uma senha única.

O container da agenda executa com UID/GID **10001:10001**. Prepare o volume no host:

```bash
mkdir -p data/ab
sudo chown -R 10001:10001 data/ab
docker compose up -d --build
```

Em volumes existentes, faça backup antes de ajustar permissões. Para implantação
com `hbbs`/`hbbr` já existentes, use `compose.deploy.example.yaml`, que contém
somente a agenda. Consulte o [guia de compatibilidade](docs/COMPATIBILIDADE-RUSTDESK.md)
antes de atualizar volumes ou credenciais.
As imagens de agenda do upstream não contêm as alterações desta continuação; construa o
código ou publique uma imagem própria antes de usá-las.

## Instalação e atualização pelo GitHub

O código desta continuação está em `https://github.com/mk-tecnologia/RUSTBOOK`.
Para preparar uma instalação com `hbbs`/`hbbr` já existentes:

```bash
git clone https://github.com/mk-tecnologia/RUSTBOOK.git
cd RUSTBOOK
cp compose.deploy.example.yaml compose.deploy.yaml
```

Configure `.env`, proxy TLS e o volume conforme o
[guia de implantação isolada](docs/COMPATIBILIDADE-RUSTDESK.md#implantação-com-hbbshbbr-existentes).
O caminho `./data` é relativo a este diretório: em uma migração de outro checkout,
configure o volume para o banco existente antes de iniciar o container. Um volume
vazio cria uma instalação nova, sem os usuários e agendas anteriores.

Para atualizar um checkout deste repositório:

```bash
git status --short
git pull --ff-only origin main
docker compose -f compose.deploy.yaml config --quiet
docker compose -f compose.deploy.yaml build
```

Use um checkout sem alterações versionadas locais. Antes de iniciar a nova imagem,
homologue o commit numa cópia do banco e faça backup consistente dos dados,
configuração e imagem anterior, conforme [operação e rollback](docs/OPERACAO.md).
Então atualize **somente a agenda**:

```bash
docker compose -f compose.deploy.yaml up -d --no-deps --no-build address-book
docker compose -f compose.deploy.yaml logs --tail=100 address-book
curl --fail http://127.0.0.1:21114/api/version
```

Ajuste a URL ao endereço configurado e confirme login, leitura e gravação da
agenda com o cliente real. As migrações são automáticas ao iniciar; um rollback
pode exigir restaurar o backup. Preserve a chave JWT e as permissões do volume.
Para implantações reproduzíveis, registre o SHA com `git rev-parse HEAD` e use o
mesmo commit homologado em cada servidor. O repositório não atualiza servidores
automaticamente nem publica imagens Docker prontas.

## Configuração

Copie `config.toml.example` para `config.toml` se preferir arquivo. Variáveis de
ambiente têm precedência. Não versione senhas, tokens, `.env` ou bancos.

| Variável | Chave TOML | Padrão |
| --- | --- | --- |
| `RUSTDESK_AB_PORT` | `port` | `21114` |
| `RUSTDESK_AB_DB_PATH` | `db_path` | `data/db.sqlite3` |
| `RUSTDESK_AB_JWT_SECRET` | `jwt_secret` | Aleatório por inicialização; configure chave persistente em produção |
| `RUSTDESK_AB_ADMIN_USERNAME` | `admin_username` | `admin` |
| `RUSTDESK_AB_ADMIN_PASSWORD` | `admin_password` | Sem padrão; obrigatório na primeira execução |
| `RUSTDESK_AB_TOKEN_EXPIRY_HOURS` | `token_expiry_hours` | `168`; intervalo `1..720` |
| `RUSTDESK_AB_CORS_ORIGINS` | `cors_origins` | Vazio; origens exatas separadas por vírgula na variável |
| `RUSTDESK_AB_ALLOW_UNAUTHENTICATED_DEVICE_EVENTS` | `allow_unauthenticated_device_events` | `false` |

CORS vazio mantém o console na mesma origem. Clientes nativos não precisam de
CORS. Para outra origem web, configure por exemplo `https://console.example.com`.
A API limita corpos JSON a 2 MiB. Login permite até 10 tentativas por conta e 120
no processo por minuto, além de limitar o trabalho Argon2 concorrente. Use TLS e
um limite adicional por IP no proxy; os limites em memória reiniciam com o serviço
e não são compartilhados entre réplicas.

**Eventos de dispositivos exigem autenticação por padrão.** Clientes que enviam
heartbeat/sysinfo/auditoria sem login podem precisar da opção de compatibilidade.
Ela deve ser habilitada somente com restrição de rede/proxy, pois eventos anônimos
não comprovam a identidade do dispositivo. Mesmo autenticado, o conteúdo do evento
é declarado pelo cliente e não deve ser tratado como evidência independente.

## Configuração do cliente RustDesk

Em Settings → Network, configure ID Server, Relay Server e a chave do seu `hbbs`.
Configure API Server com a URL deste serviço, por exemplo
`https://rustdesk.example.com`, e faça login pelo painel da agenda.

## API

As rotas existentes do RustDesk foram preservadas. A implementação segue os
[níveis de compartilhamento](https://github.com/rustdesk/rustdesk/blob/master/flutter/lib/common/hbbs/hbbs.dart)
e os formatos de atualização/exclusão presentes no
[código do cliente](https://github.com/rustdesk/rustdesk/blob/master/flutter/lib/models/ab_model.dart).
Isso não substitui homologação com o aplicativo real.

| Endpoint | Uso |
| --- | --- |
| `GET /api/version` | Versão pública do binário em execução, sem cache |
| `POST /api/login`, `POST /api/logout` | Criar/revogar sessão |
| `GET /api/currentUser` | Usuário atual e papel atualizado |
| `GET/POST /api/ab/personal`, `GET/POST /api/ab/shared/profiles` | Perfis; POST pessoal retorna `guid` na raiz |
| `GET/POST /api/ab/settings` | Configuração da agenda |
| `GET /api/ab`, `POST /api/ab` | Leitura/substituição legada transacional |
| `GET/POST /api/ab/peers` | `ab`, `current`, `pageSize` (1–100), `q` e `tag` |
| `POST /api/ab/peer/add/{guid}` | Inserir/upsert |
| `PUT /api/ab/peer/update/{guid}` | Atualização parcial, campo `id` obrigatório |
| `DELETE /api/ab/peer/{guid}` | Lista de IDs ou objeto `{ "ids": [...] }` |
| `GET/POST /api/ab/tags/{guid}` | Tags; POST retorna lista |
| `POST /api/ab/tag/add/{guid}` | Adicionar tag |
| `PUT /api/ab/tag/rename/{guid}`, `PUT /api/ab/tag/update/{guid}` | Nome/cor da tag |
| `DELETE /api/ab/tag/{guid}` | Lista de nomes ou objeto `{ "names": [...] }` |
| `GET /api/peers`, `GET /api/device-group/accessible` | Inventário e grupos de dispositivos acessíveis |
| `GET /api/users?accessible=&status=1` | Perfis ativos acessíveis no formato do cliente |
| `GET/POST /api/users`, `PUT/DELETE /api/users/{id}` | Administração de usuários |
| `GET/POST /api/groups`, `PUT/DELETE /api/groups/{id}` | Administração de grupos |
| `GET /api/audit-log` | Admin; `current` e `pageSize` |
| `POST /api/heartbeat`, `POST /api/system/heartbeat` | Heartbeat |
| `POST /api/system/sysinfo`, `POST /api/audit` | Informações/eventos |

Permissões: `1` leitura, `2` leitura/escrita, `3` controle total. O proprietário tem
controle; as operações atuais de peers/tags exigem ao menos `2`. Ser administrador
não dá acesso implícito a agendas alheias. Alteração de senha ou status revoga
sessões; alterações de papel são aplicadas na próxima requisição. Revogação não
cancela requisições que já estavam em andamento.

## Verificação

```bash
python3 scripts/check_version.py
npm --prefix web test
npm --prefix web run build
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

A CI executa essas verificações. A suíte cobre permissões, sessões, atualização
parcial, rollback, paginação, auditoria e migrações. Consulte o resultado das execuções na aba Actions do repositório.
Proteções de branch precisam ser configuradas separadamente no GitHub.

## Documentação

- [Compatibilidade com o cliente e implantação isolada](docs/COMPATIBILIDADE-RUSTDESK.md)
- [Operação, atualização e backup](docs/OPERACAO.md)
- [Análise técnica e estado dos achados](docs/ANALISE-TECNICA.md)
- [Padrões de layout](docs/LAYOUT.md)
- [Versionamento e GitHub](docs/VERSIONAMENTO.md)
- [Contribuição](CONTRIBUTING.md)
- [Changelog](CHANGELOG.md)

## Licença e atribuição

Derivado de [ds4a/rustdesk-address-book](https://github.com/ds4a/rustdesk-address-book),
commit base `ca39fa3`, com histórico e autoria originais preservados.
O manifesto upstream declara **AGPL-3.0**; esta continuação mantém essa licença.
Consulte [LICENSE](LICENSE) e preserve os avisos e a atribuição ao redistribuir.
