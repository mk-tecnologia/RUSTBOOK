# Operação e atualização

## Antes de atualizar uma instalação existente

1. Pare o serviço da agenda e faça backup do banco e dos arquivos WAL/SHM que
   existirem. Com o processo parado, copiar todo o diretório de dados preserva o
   conjunto. Guarde também configuração e chave JWT em local protegido.
2. Teste a atualização numa cópia do banco, sem apontar testes para dados reais.
3. Verifique se há agendas pessoais duplicadas, usando um cliente SQLite:

   ```sql
   SELECT owner_id, COUNT(*) FROM address_books
   WHERE is_personal = TRUE GROUP BY owner_id HAVING COUNT(*) > 1;
   ```

   A migração 008 recusa duplicatas sem excluir dados. Se existirem, consolide
   peers/tags e referências com um plano específico para os registros afetados.
   Não apague agendas automaticamente para satisfazer o índice.
4. Defina uma chave JWT persistente de pelo menos 32 bytes. Na primeira instalação,
   defina senha administrativa de 12–1024 bytes.
5. Garanta permissão de escrita no diretório SQLite. No container, o usuário é
   `10001:10001`; ajuste o bind mount no host. O WAL exige escrita no diretório,
   não apenas no arquivo `.sqlite3`.
6. Inicie a nova versão. As migrações são embutidas e aplicadas em transações.
   Faça login novamente e teste leitura, gravação, permissões e auditoria.

## Mudanças de comportamento em 0.2.0

- Tokens anteriores à tabela de sessões deixam de funcionar; todos devem entrar
  novamente. Logout revoga a sessão; troca de senha/status revoga sessões do usuário.
- Compartilhamentos de leitura não podem gravar peers/tags. Regras inválidas ou `0`
  não concedem acesso; revise registros antigos se necessário.
- Heartbeat, sysinfo e audit exigem Bearer token. Para clientes anônimos, a opção
  `RUSTDESK_AB_ALLOW_UNAUTHENTICATED_DEVICE_EVENTS=true` é uma exceção explícita;
  restrinja a entrada no proxy/rede e homologue o cliente antes de habilitá-la.
- CORS não é mais aberto. A interface embutida funciona na mesma origem;
  configure origens exatas para frontends separados.
- `pageSize` deve estar entre 1 e 100, e `current` começa em 1.
- Atualizações de peer exigem `id`; exclusões aceitam o formato de lista do cliente
  e o formato de objeto da interface.
- Erros de rota `/api/*` retornam 404 JSON, sem mascarar a falha com HTML.

## Migrações

A tabela `_migrations` do upstream é mantida. `_migration_sources` armazena a fonte
SQL de cada migração para detectar edições futuras; registros herdados ganham a
fonte na primeira atualização. Não é possível provar retroativamente que o SQL
original executado era idêntico, pois o upstream não armazenava essa informação.

Não edite migrações aplicadas. Adicione um novo arquivo sequencial e registre-o na
lista embutida em `src/db.rs`. Falhas revertem a migração atual e não marcam sua
aplicação; migrações anteriores que já terminaram continuam aplicadas.

## Produção

- Terminar TLS no proxy e encaminhar somente o tráfego necessário à porta 21114.
- Aplicar limites por IP no proxy, além dos limites por conta/processo do servidor.
- Não compartilhar o arquivo SQLite por filesystem de rede nem escalar vários
  containers escritores sem revisar a arquitetura. Medir contenção antes de mudar
  para PostgreSQL.
- Definir retenção de auditoria conforme o volume; ainda não há expurgo automático.
- Monitorar logs, respostas 5xx, espaço em disco e duração de backup/restauração.
- Não registrar senhas ou tokens. Eventos recebidos são dados do cliente; o IP de
  eventos também é informado pelo cliente e não comprova a origem da conexão.
- O servidor trata SIGINT e SIGTERM para encerramento gracioso.

## Rollback

Não há migração de downgrade automática. Para retornar a um binário anterior,
pare o serviço, restaure a cópia completa do banco/configuração e só então inicie
esse binário. Não substitua o banco com o serviço em execução.

## Dependências

Os lockfiles foram atualizados após auditoria npm e RustSec. O SQLx foi atualizado
para 0.9.1 para permitir o SQLite corrigido (`libsqlite3-sys` 0.40.0). MySQL,
PostgreSQL, Chrono e funcionalidades HTTP não usadas foram removidos da resolução.
O banco permanece SQLite; a migração da biblioteca não muda o motor nem exige
exportação para outro banco. Faça backup e homologue a atualização numa cópia.

As auditorias não encontraram avisos conhecidos nos lockfiles finais nesta
verificação. Esse resultado é temporal e não equivale a uma garantia de segurança.
A CI bloqueia alertas npm de severidade alta/crítica e executa RustSec com
`cargo-audit`. Para repetir localmente, instale `cargo-audit` e execute `cargo audit`
ao atualizar dependências ou preparar releases.

## Limites da validação

Os testes automatizados usam SQLite isolado e requisições HTTP em memória. Eles
não substituem testes com RustDesk desktop/mobile, carga concorrente, proxy TLS
ou o ambiente Docker de destino. O token web continua no localStorage; mantenha a
interface livre de conteúdo HTML não confiável e revise XSS antes de expô-la.

## Verificações desta alteração

Na validação de 2026-11-21:

- 19 testes Rust aprovados com SQLx 0.9.1 e SQLite atualizado.
- Clippy aprovado com `-D warnings`; rustfmt, versões e YAML verificados.
- Build TypeScript/Vite aprovado; build Docker Linux/ARM64 aprovado com Node 22.
- Auditorias npm e RustSec sem vulnerabilidades ou avisos conhecidos nos lockfiles finais.
- Teste HTTP no binário e na imagem: frontend, 404 JSON, CORS, login, agenda,
  inserção, pesquisa, validação de paginação, exclusão e revogação no logout.
- Runtime Docker confirmado como UID/GID 10001; sessão preservada após reinício
  com chave JWT persistente; encerramento por SIGTERM verificado.
- Container temporário removido; imagem local `rustbook-address-book:0.2.0` mantida.

A validação visual pelo navegador não foi concluída porque o conector falhou ao
carregar um arquivo interno. Não foram homologados clientes RustDesk reais,
implantação com proxy TLS ou cenários de carga. Nada foi publicado no GitHub ou
em um registro de imagens.

## Complemento de compatibilidade do contexto de implantação

As consultas POST sem corpo e os contratos da aba Dispositivos foram incorporados
após a revisão de `CONTEXTO_CORRECOES.md`. A suíte passou a 23 testes, com Clippy e
build web aprovados. O verificador HTTP foi executado em servidor temporário com
admin, usuário comum e dispositivo sintético; os testes não alteraram a instalação
informada. O aviso incorreto de JWT não aparece quando a chave vem do ambiente.

Use o [guia de compatibilidade](COMPATIBILIDADE-RUSTDESK.md) e
`compose.deploy.example.yaml` quando hbbs/hbbr já existem. A implantação atual não
foi atualizada automaticamente. Grupos de dispositivos e ACLs por dispositivo
continuam pendentes; não confunda respostas vazias de compatibilidade com essas
funcionalidades completas.

O complemento também foi validado em uma imagem Linux amd64: build de produção e
verificador HTTP aprovados, runtime 10001:10001 com privilégios reduzidos e
container removido ao terminar. Imagens base foram fixadas por digest após essa
validação. A imagem de teste ficou local como `rustbook-address-book:0.2.0-amd64`;
não foi publicada nem enviada ao servidor.

## Versão no cabeçalho e preferência de tema

O painel consulta `GET /api/version`, que retorna a versão do binário compilado
(`CARGO_PKG_VERSION`) e `Cache-Control: no-store`. Não depende da versão de um
frontend eventualmente em cache. O badge mostra "Version unavailable" se o servidor
não disponibiliza essa rota. Para a alteração aparecer em uma instalação existente,
é preciso compilar a interface, reconstruir o binário/imagem e atualizar a implantação.
Nenhuma atualização no servidor foi executada nesta alteração.

O seletor de tema oferece System, Light e Dark. System segue a preferência do
sistema inclusive quando ela muda com a página aberta; Light/Dark têm prioridade.
Escolhas explícitas antigas são preservadas. Se o armazenamento do navegador estiver
bloqueado, a escolha ainda funciona na aba atual, mas não persiste após recarregar.

Validação: nove testes de tema aprovados também em Node 22, 24 testes Rust aprovados,
build web aprovado e Clippy sem avisos. Os testes de tema exercitam o store real com
DOM/eventos simulados; a validação visual em navegador continua pendente.
