# Configuração e API

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
