# Changelog

Histórico desta continuação de `ds4a/rustdesk-address-book`. A versão herdada do
manifesto upstream é `0.1.0`; não são inventadas releases anteriores do upstream.

## [Unreleased]

- Instalador escolhe a build pela data de publicação, sem depender da ordem
  da lista de releases retornada pelo GitHub.

## [0.5.1] - 2026-11-21

### Fixed

- Janelas de edição/criação centralizadas horizontal e verticalmente, inclusive
  após o reset de estilos do Tailwind.
- Ações de usuários agrupadas em botões consistentes com ícone e texto:
  Edit, Disable/Enable e Delete. Papel e status são indicadores sem ações ocultas.
- Configuração de presença acessível no painel, com indicação explícita de
  servidor não configurado ou inacessível, em vez de apenas Unknown na lista.

### Added

- Configuração de presença por instalação salva no banco, sem IP fixo, com teste
  de protocolo antes de salvar e atualização imediata da lista.
- Configuração restrita a administradores e opção de restaurar o padrão da
  implantação. Backup e atualização preservam a configuração junto ao banco.

## [0.5.0] - 2026-11-21

### Fixed

- Login no aplicativo RustDesk: descoberta de provedores sem OIDC e perfil via POST.
- Nome de login preservado separado do nome de exibição nos perfis de autenticação.
- Relatórios antigos ou ausentes da API deixam de indicar falsamente Offline.
- A lista atualiza a presença a cada 30 segundos e ao voltar à janela.
- Falhas de consulta indicam presença desconhecida, preservando os dispositivos.

### Added

- Edição de usuários no painel, incluindo perfil, papel, status e senha opcional.
- Confirmação de senha ao criar usuários ou trocar a senha, sem enviar a confirmação à API.
- Consulta nativa de presença ao hbbs, respeitando as permissões existentes.
- Estados Online/Offline confirmados pelo servidor de ID, atividade recente e
  presença desconhecida, com descrição da origem e do último relatório.
- Opção `--hbbs HOST:PORT` no instalador, persistida nas próximas atualizações
  e protegida pelo backup e rollback da configuração.

## [0.4.1] - 2026-11-21

### Added

- Organização dos dispositivos em grupos expansíveis por cliente ou local.
- Criação de tags com cor e seleção de múltiplas tags ao incluir ou editar peers.
- Busca, filtro por tag e paginação independente por grupo.
- Migração aditiva com preservação de peers, tags e grupos administrativos existentes.

### Fixed

- Editar grupo ou tags de um peer com plataforma desconhecida preserva esse valor,
  sem preencher Windows automaticamente.
- Escritas de peers e sincronização legada adquirem o bloqueio de escrita antes de
  validar grupos ou ler vínculos, evitando disputa de promoção de transação SQLite.

Inclui a organização por grupos e tags descrita em 0.4.0. Esta é a primeira
publicação de imagens dessa funcionalidade; a execução de preparação de 0.4.0
foi substituída antes da publicação.

## [0.4.0] - preparação da funcionalidade

### Added

- Grupos de dispositivos por agenda, com criação, edição e exclusão, organizados
  em linhas expansíveis com contagem e paginação independente por pasta.
- Seleção de grupo e múltiplas tags ao adicionar ou editar um dispositivo.
- Criação de tags com cor diretamente no formulário do dispositivo e na lateral.
- Busca e filtro por tag aplicados aos grupos e seus dispositivos.
- Estado online/offline para admins quando há um relato de inventário disponível;
  status desconhecido permanece explícito quando não há acesso ao relato.

### Fixed

- O formulário deixa de enviar tags vazias obrigatoriamente e permite atribuir
  tags aos peers existentes.
- Cadastro duplicado pelo painel retorna conflito sem apagar nome, tags ou grupo.
- Atualizações nativas parciais e substituições legadas preservam a pasta de cada
  RustDesk ID quando o cliente não informa um grupo.

### Compatibility

- Migração aditiva `009_peer_groups.sql`; peers existentes permanecem sem grupo.
- Excluir uma pasta desassocia seus peers, preservando dispositivos e tags.
- Pastas respeitam as permissões da agenda e não concedem acesso a outras agendas.
- Grupos administrativos de usuários e seus vínculos são preservados separadamente.
- Faça backup antes de atualizar; um rollback para 0.3.0 deve restaurar o banco
  anterior, conforme o procedimento de operação.

## [0.3.0] - 2026-11-21

### Added

- Imagens Docker prontas para Linux amd64 e arm64, publicadas pela CI após os
  testes, com manifesto SHA-256 e identificação do commit compilado.
- Release de produto `v0.3.0` e builds automáticos identificados por commit.
- API de versão com `version` e `commit`; o cabeçalho exibe ambos e atualiza a
  consulta a cada minuto e ao retornar à janela.

### Performance

- O instalador baixa e verifica a imagem pronta, sem compilar no servidor.
- Instalação saudável no mesmo commit termina sem download de imagem ou reinício.
- Camada independente de dependências Rust e cache de build por arquitetura.
- Compilação local da `main` continua disponível com a opção explícita `--build`.

### Compatibility

- Preserva o fluxo de adoção de agendas Compose, dados, configuração e credenciais.
- Backup offline e rollback da imagem e do banco continuam ativos nas atualizações.
- Não adiciona migrações de banco em relação ao desenvolvimento `0.2.0`.
- Clientes de `/api/version` que usam apenas `version` continuam compatíveis.
- Ao atualizar uma instalação antiga, faça login novamente se as sessões expirarem
  e valide os clientes RustDesk conforme o guia de operação.

## [0.2.0] - desenvolvimento anterior, sem tag ou release

As alterações abaixo foram publicadas em commits de desenvolvimento antes da
primeira release de produto. Não houve uma release `v0.2.0` no GitHub.

### Fixed — atualização de instalações existentes

- `install latest` reconhece a agenda Compose existente e preserva seu volume,
  projeto, portas, configuração e credenciais, sem exigir importação manual.
- Backup antes das migrações e rollback com imagem local anterior, WAL/SHM e
  permissões originais, inclusive quando a versão antiga não tem `/api/version`.
- Operações de Compose limitadas à agenda; wrapper `manage.sh` usa os arquivos
  e o projeto corretos nas instalações novas e reconhecidas.

### Added

- Instalador Linux por comando único (`install latest` / `update latest`), com
  credenciais persistentes, backup offline e tentativa de rollback automático.
- Importação inicial de agenda existente e regressões de segurança do instalador.
- Análise técnica, guia de layout e regras de contribuição e versionamento.
- CI para build web, testes Rust, Clippy e consistência de versões.
- Modelos de pull request e issues, EditorConfig e verificador de versões.

### Changed

- Seletor de tema substituído por controle segmentado com ícones, destaque da
  seleção e navegação nativa por teclado.
- README focado em instalação e uso; desenvolvimento, configuração e API movidos
  para documentação específica.
- Versão do pacote web alinhada à versão `0.1.0` do produto.
- Arquivos locais do IDE e segredos de ambiente ignorados pelo Git.

### Fixed

- Declaração de `ApiError` compatível com `erasableSyntaxOnly`, permitindo o
  typecheck do frontend sem desabilitar a configuração estrita do TypeScript.

### Security

- Dependências npm atualizadas para remover os nove alertas identificados na auditoria.
- Dependências Rust atualizadas; SQLx 0.9.1 e SQLite corrigido, drivers não usados
  removidos e timestamps JWT calculados com a biblioteca padrão. Auditoria RustSec
  final sem vulnerabilidades ou avisos conhecidos.

- Sessões persistidas com identificador único, revogação por logout/senha/status e
  autorização baseada no papel atual do usuário.
- Permissões de leitura/escrita nas agendas, CORS por origem explícita, limite de
  login e concorrência Argon2, e senha inicial obrigatória.
- Eventos exigem autenticação por padrão; exceção de compatibilidade é explícita.

### Fixed

- Sincronização legada e gravações relacionadas com rollback transacional.
- Atualização real de peers, exclusões no formato do cliente e limpeza de tags vazias.
- Consultas de tags em lote, perfis compartilhados únicos e agenda pessoal idempotente.
- Paginação, busca e filtro completos; leitura de auditoria implementada.
- Tratamento de erros, submissões repetidas, labels, diálogo nativo, tema e navegação móvel.
- Build TypeScript, build Docker com tipos verificados e runtime sem usuário root.
- Migrações SQL embutidas e aplicadas atomicamente sem divisão por ponto e vírgula.

### Compatibility

- Preparação de `0.2.0`, sem publicação: tokens antigos expiram na atualização,
  eventos anônimos e CORS aberto deixam de ser padrão. Consulte `docs/OPERACAO.md`.

### Compatibility — contexto da instalação

- Consultas POST sem corpo na agenda; perfil pessoal na raiz e tags como lista,
  preservando GETs do painel e as verificações de acesso.
- Rotas da aba Dispositivos e perfis acessíveis: inventário exclusivo de admins,
  usuários comuns restritos ao próprio perfil e grupos de dispositivos vazios.
- Quatro novas regressões de contrato/autorização, executadas pela CI, e verificador
  HTTP reutilizável sem acesso direto ao SQLite.
- Compose de agenda isolada, limite configurável de jobs no build, imagens base
  fixadas por digest após validação amd64 e exclusão de backups.
- Aviso de chave JWT emitido apenas no fallback aleatório, uma vez por inicialização.

### Interface — versão e tema

- Cabeçalho exibe a versão do binário retornada por `GET /api/version`, com cache
  desabilitado e indicação explícita quando não é possível confirmar a versão.
- Seleção de tema System/Light/Dark: modo automático preservado, mudanças do sistema
  acompanhadas em tempo real e escolhas manuais persistidas entre acessos/abas.
- Inicialização do tema antes da montagem das telas, color-scheme dos controles
  nativos e cabeçalho com quebra de linha em telas estreitas.
- Nove regressões de tema incluídas na CI e teste do endpoint público de versão.
