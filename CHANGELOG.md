# Changelog

As mudanças desta continuação do projeto serão registradas aqui. A versão `0.1.0`
vem do manifesto upstream; este arquivo não declara uma release própria publicada.

## [Unreleased]

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
