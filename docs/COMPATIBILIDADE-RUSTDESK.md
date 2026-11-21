# Compatibilidade com o cliente RustDesk

Correções incorporadas a partir de `CONTEXTO_CORRECOES.md`, registro técnico de
21/11/2026 da instalação baseada no upstream `ca39fa3`. O patch mencionado no
registro não foi aplicado diretamente: esta continuação já alterou os mesmos
arquivos para corrigir segurança e integridade. Os contratos foram integrados aos
handlers atuais e aos testes em `src/tests.rs`.

## Agenda: métodos e formatos

As consultas POST não exigem corpo nem Content-Type. Autenticação e regras de
acesso são as mesmas do GET. Parâmetros de paginação e agenda ficam na URL.

| Rota | GET do painel | POST do cliente |
| --- | --- | --- |
| `/api/ab/personal` | `{ "data": { "guid": "...", ... } }` | Perfil no objeto principal, com `guid` |
| `/api/ab/settings` | Objeto com `max_peer_one_ab` | Mesmo objeto |
| `/api/ab/shared/profiles` | Objeto com `data` e `total` | Mesmo objeto |
| `/api/ab/peers` | Objeto com `data` e `total` | Mesmo objeto, com `ab`, `current` e `pageSize` na URL |
| `/api/ab/tags/{guid}` | Objeto com `data` e `total` | Lista JSON de tags |

A consulta pessoal pode criar a agenda quando ainda não existe. Sucessos de
mutações de peers/tags usam corpo vazio com HTTP 200; exclusões aceitam arrays do
cliente e objetos do painel. Atualizações parciais de peers, revogação de sessões
e transações já estão cobertas pela suíte anterior.

## Aba Dispositivos

| Consulta | Comportamento |
| --- | --- |
| `GET /api/peers` | Admin recebe inventário paginado; usuários comuns recebem `data: []`, `total: 0` |
| `GET /api/device-group/accessible` | Autenticado; retorna lista vazia porque grupos/ACLs do inventário nativo não estão implementados |
| `GET /api/users?accessible=&status=1` | Admin recebe perfis ativos; usuário comum recebe somente seu perfil ativo |
| `GET /api/users` sem `accessible` | Mantém o contrato administrativo existente; usuários comuns recebem 403 |

Cada dispositivo retorna ID RustDesk textual, `info` com `device_name`, `os`,
`version` e `username`, além de `status`, `user`, `user_name`, `device_group_name`
e `note`. Sem associação correspondente no banco, os campos ficam vazios.
`os` usa a plataforma como alternativa quando não há descrição de sistema.

O status online é aproximado pela existência de relato nos últimos 120 segundos.
Datas futuras ou inválidas não contam como online. Os parâmetros `accessible` e
`status` enviados pelo cliente não criam ACLs nem um estado de ativação inexistente
no esquema; o admin vê dispositivos online e offline, conforme a paginação.

Perfis acessíveis usam `name` como username de login e `display_name` como nome de
exibição. Não incluem hash de senha, email, papel administrativo ou dados de outras
contas para usuários comuns. `status=0` retorna vazio nesse contrato, pois contas
inativas não são perfis acessíveis. O GET administrativo preserva seus campos.

As três consultas exigem sessão válida. Paginação usa `current >= 1` e
`1 <= pageSize <= 100`. Rotas desconhecidas `/api` e `/api/...` retornam 404 JSON;
navegação fora da API continua usando o fallback do painel.

## Implantação com hbbs/hbbr existentes

Para o instalador por comando único e migração de instalações existentes, consulte
[instalação](INSTALACAO.md). Esta seção descreve a alternativa com Compose manual.

Use `compose.deploy.example.yaml` como base e copie para `compose.deploy.yaml`.
O template contém somente a agenda, usa o Dockerfile local e preserva o caminho
`./data/db.sqlite3` descrito no registro. Não substitua a configuração existente
antes de comparar volumes, chave JWT e permissões.

Crie um `.env` protegido com as variáveis abaixo, preenchendo os segredos:

```dotenv
BIND_IP=127.0.0.1
RUSTDESK_AB_JWT_SECRET=
RUSTDESK_AB_ADMIN_PASSWORD=
CARGO_BUILD_JOBS=2
```

Ajuste BIND_IP para o IP interno do host se o cliente acessar diretamente pela LAN.
Defina uma chave aleatória persistente com pelo menos 32 bytes e uma senha inicial
única de 12–1024 bytes. O Compose recusa segredos ausentes. Em instalação existente,
preserve a chave JWT; novos logins ainda serão necessários devido ao controle de
sessões de 0.2.0. A senha de ambiente não redefine usuários já cadastrados.

Faça backup consistente com o serviço parado, guarde uma cópia do Compose e anote
a imagem anterior. O container novo roda como `10001:10001`: o diretório `data/`
e seus arquivos precisam ser graváveis por esse usuário. Restrinja `.env` a 0600
e o diretório de dados a 0700. Não copie nem versione backups e segredos.

Após configurar e homologar numa cópia:

```bash
docker compose -f compose.deploy.yaml config --quiet
docker compose -f compose.deploy.yaml build
docker compose -f compose.deploy.yaml up -d --no-build
```

Esses comandos se referem ao template de agenda isolada. `docker-compose.yml` da
raiz contém a stack completa e não deve ser usado para substituir relay/ID já
existentes. Nenhum comando de implantação foi executado no servidor informado.

A ingestão de heartbeat/sysinfo exige login por padrão nesta continuação. Se o
cliente da instalação envia eventos anônimos, a exceção
`RUSTDESK_AB_ALLOW_UNAUTHENTICATED_DEVICE_EVENTS=true` precisa ser escolhida
explicitamente e protegida pela rede/proxy. Consulte [operação](OPERACAO.md).

O build aceita `CARGO_BUILD_JOBS` (padrão 2) para limitar concorrência. O ajuste de
Rust 1.83 para `rust:1-bookworm` não é tratado como prova de incompatibilidade do
compilador antigo. O template faz build local e não depende do acesso anônimo à
imagem GHCR upstream. As imagens base estão fixadas pelos digests dos índices validados; atualize os
digests deliberadamente e repita a homologação ao preparar releases.

## Verificador para uma instalação configurada

```bash
python3 scripts/verify_client_api.py --base-url http://endereco-interno:21114 --username admin
```

O script pede a senha sem exibi-la e sem colocá-la nos argumentos do processo.
Como alternativa, use um token existente na variável `RUSTDESK_VERIFY_TOKEN` e
omita `--username`. Não inclua credenciais no endereço. O script não segue
redirecionamentos com o token.

Ele verifica consultas GET/POST da agenda, envelopes, autenticação, perfis e
inventário. Não acessa SQLite nem cria dispositivos ou contas. Um login de teste
gera auditoria e a sessão é encerrada ao terminar. A consulta pessoal pode criar a
agenda do usuário se ela ainda não existir. Tokens fornecidos externamente não
são revogados. A presença de dados reais e a UI do aplicativo precisam de
homologação separada; os testes Rust usam registros sintéticos para cobrir os
casos que uma instalação vazia não permite verificar.

O verificador aceita `RUSTDESK_VERIFY_PASSWORD` para testes automatizados isolados;
para uso manual prefira o prompt. Ele não foi executado contra o servidor informado.

## Validação e limites

A CI executa as regressões de consultas POST, formatos GET/POST, paginação,
isolamento de contas, perfis ativos e respostas da aba Dispositivos, junto com as
correções anteriores. Testes usam SQLite isolado, sem inserir dados no servidor.

Continuam pendentes grupos de dispositivos completos, associação de proprietários,
ACLs por dispositivo e importação dos IDs de hbbs. A lista reflete apenas relatos
recebidos pela agenda. A versão exata do aplicativo usada na instalação do registro
não foi informada; não se declara homologação de uma versão específica. O sucesso
das consultas não prova todos os fluxos de escrita ou a interface do cliente real.

Referências do cliente consultadas:
[agenda](https://github.com/rustdesk/rustdesk/blob/master/flutter/lib/models/ab_model.dart),
[dispositivos](https://github.com/rustdesk/rustdesk/blob/master/flutter/lib/models/group_model.dart)
e [payloads](https://github.com/rustdesk/rustdesk/blob/master/flutter/lib/common/hbbs/hbbs.dart).

### Resultado desta incorporação

- 23 testes Rust, Clippy com avisos tratados como erro e build web aprovados.
- Build de produção Linux amd64 aprovado com `CARGO_BUILD_JOBS=2`.
- Verificador HTTP aprovado no binário local com admin/usuário comum e dispositivo
  fictício, e novamente na imagem amd64.
- Runtime confirmado em Linux/amd64 como UID/GID 10001 com capabilities removidas
  e `no-new-privileges`; container temporário removido após o teste.
- `.env`, backups e bancos do servidor não foram acessados. A instalação informada
  não foi alterada, reiniciada ou substituída.

## Organização da agenda em 0.4.0

O console tem pastas de dispositivos por agenda, independentes dos grupos
administrativos de usuários e do inventário nativo. Consulte [grupos e tags](GRUPOS-E-TAGS.md).
A associação opcional `group_id` não altera os campos usados pelo cliente. O
[parser de peers do cliente](https://github.com/rustdesk/rustdesk/blob/master/flutter/lib/models/peer_model.dart)
consulta os campos conhecidos e não depende desse campo adicional. Ao atualizar
um peer sem `group_id`, a pasta é preservada; o POST legado completo também
preserva a pasta por RustDesk ID retido. Os testes incluem esses contratos,
permissões, paginação, rollback de tags inválidas e exclusão de grupos sem apagar peers.

## Login no cliente nativo

`GET /api/login-options` é público e retorna `[]`, pois este servidor oferece
senha e não possui provedores OIDC. Essa descoberta precede o formulário de login
nos clientes recentes; responder 404 impede a tentativa de autenticação.
`POST /api/login` emite a sessão. `GET` e `POST /api/currentUser` retornam o mesmo
perfil e exigem sessão válida. `name` contém o username de login; `display_name`
contém o nome de exibição. Logout e alterações de senha continuam revogando sessões.

No painel Users, use Edit para alterar nome, email, papel, ativação ou senha.
O username de login permanece fixo. As senhas exigem confirmação no formulário;
deixar os dois campos vazios ao editar preserva a senha atual. Salvar apenas o
perfil não revoga sessões por enviar um status inalterado.

A presença da agenda web usa [consulta ao hbbs](PRESENCA.md). O status do
inventário nativo `/api/peers` mantém o contrato de relatos descrito acima.
