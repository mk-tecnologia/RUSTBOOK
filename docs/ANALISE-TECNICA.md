# Análise técnica da base

Base analisada: upstream `ds4a/rustdesk-address-book`, commit `ca39fa3`.
Esta é uma revisão estática, sem benchmark de carga ou homologação com clientes RustDesk.

## Estado após as correções de 0.2.0

Os achados abaixo documentam a revisão do commit upstream. As seguintes correções
já foram implementadas nesta continuação:

- Autorização centralizada por leitura/escrita, inclusive compartilhamento por grupo.
- Sessões persistidas, logout efetivo, revogação por senha/status e papel atualizado.
- Transações para sincronização legada, peers/tags e alterações de usuários.
- Atualização parcial real de peers, com ID obrigatório e formato de exclusão do cliente.
- Pesquisa/filtro/paginação no servidor e controles na interface.
- Auditoria paginada para admins; erros distinguem-se de lista vazia.
- Migrações embutidas com transação e detecção de alteração de fontes.
- Índice de agenda pessoal única e criação idempotente; migração recusa duplicatas.
- Tags carregadas em lotes e perfis compartilhados sem consultas por proprietário.
- Credenciais iniciais obrigatórias, CORS configurável e autenticação de eventos.
- Dependências Rust/npm auditadas e atualizadas, com SQLx 0.9.1 e SQLite corrigido.
- Argon2 fora dos workers assíncronos, concorrência limitada e limite de login.
- Tema manual corrigido, navegação responsiva e componentes de erro/paginação/modal.

Continuam dependentes de homologação: protocolo nas versões reais dos clientes,
controle de identidade física de dispositivos, retenção de auditoria, avaliação de
carga e validação visual completa. O console ainda não edita membros de grupos nem
compartilhamentos; não foram acrescentados como novas funcionalidades neste trabalho.
Veja [operação e atualização](OPERACAO.md) para as mudanças de comportamento.

## Correções do contexto de implantação

O registro `CONTEXTO_CORRECOES.md` foi incorporado aos contratos atuais: consultas
POST sem corpo da agenda, perfil pessoal na raiz e tags como lista, além das rotas
da aba Dispositivos e dos perfis acessíveis. Os GETs do painel foram preservados.
Há quatro novos testes de regressão, totalizando 23. Inventário global continua
restrito a admins até existir associação de proprietário e ACLs reais. Consulte
[compatibilidade e implantação isolada](COMPATIBILIDADE-RUSTDESK.md).

## Decisão sobre linguagem

**Manter Rust no backend e Vue 3 com TypeScript no frontend.** Os problemas
encontrados estão nas regras de negócio, integridade e funcionalidades incompletas;
trocar a linguagem não resolve esses pontos e exige reimplementar a compatibilidade
com o cliente RustDesk.

| Camada | Implementação atual | Decisão |
| --- | --- | --- |
| API | Rust 2021, Axum 0.8, Tokio | Manter; separar regras de negócio dos handlers conforme crescer |
| Persistência | SQLx 0.8, SQLite, WAL, pool de 5 conexões | Manter inicialmente; medir contenção e volume antes de migrar |
| Autenticação | Argon2 com salt aleatório; JWT | Manter os algoritmos; corrigir ciclo de vida das sessões |
| Interface | Vue 3, TypeScript, Pinia, Vue Router | Manter; ampliar tipagem e tratamento de estados |
| Estilos | Tailwind CSS 4, tokens CSS `rd-*` | Manter; formalizar componentes e acessibilidade |
| Distribuição | Vite, frontend embutido por rust-embed, Docker | Manter; frontend precisa ser compilado antes do Rust |

Uma migração para outra linguagem só deve ser reconsiderada se a equipe não puder
manter Rust ou se houver requisito concreto incompatível com esta solução. Uma
eventual mudança de banco é uma decisão separada: múltiplas réplicas gravadoras e
contenção medida justificariam avaliar PostgreSQL. Não há evidência de carga para
recomendar isso agora.

## Fluxos e algoritmos

1. Login consulta o usuário ativo, verifica Argon2 e emite JWT com usuário e papel.
2. Agenda pessoal é criada sob demanda. Compartilhamentos são consultados por
   usuário ou associação a grupos.
3. Peers usam CRUD e upsert com chave única `(ab_guid, rustdesk_id)`.
4. Listagem usa `LIMIT/OFFSET`; tags são carregadas em uma consulta por peer.
5. A sincronização legada recebe JSON e substitui os registros da agenda.
6. Heartbeats atualizam dispositivos; auditoria recebe eventos do cliente.

O custo mais visível é o padrão N+1: a página de peers faz uma consulta para cada
peer, além de acesso, listagem e contagem. A leitura legada e os perfis
compartilhados também repetem consultas. Buscar associações em lote e agrupá-las
por identificador elimina essas idas adicionais ao banco. A pesquisa na interface
percorre apenas os peers carregados, e não toda a agenda.

## Achados priorizados

| Prioridade | Evidência no código | Consequência e ação recomendada |
| --- | --- | --- |
| P0 — autorização | `src/routes/peers.rs::resolve_ab_guid` verifica existência de compartilhamento, sem usar `rule`; é reutilizado em gravações de peers e tags | Compartilhamento não distingue leitura e escrita. Centralizar a autorização por operação e testar usuário, grupo e proprietário |
| P0 — integridade | `src/routes/ab.rs::update_ab_legacy` apaga e reinsere dados usando o pool, sem transação, apesar do comentário sobre atomicidade | Falha parcial pode deixar a agenda incompleta. Executar a substituição em uma transação e testar rollback |
| P0 — sessões | `src/auth/middleware.rs` só valida JWT; `src/routes/auth.rs::logout` retorna sucesso sem revogar | Logout, bloqueio e remoção de papel não invalidam imediatamente tokens existentes. Definir revogação e verificar usuário ativo/papel atual |
| P1 — entrada de eventos | `src/routes/system.rs` não exige identidade em heartbeat, sysinfo ou audit | Eventos e dispositivos podem ser forjados. Definir identificação de dispositivos e limites preservando o protocolo dos clientes |
| P1 — implantação | `src/config.rs` aceita senha inicial `admin`; `src/main.rs` permite CORS para qualquer origem | Exigir configuração segura para produção e configurar origens; documentar TLS no proxy |
| P1 — paginação | `src/routes/peers.rs::get_peers` não limita `pageSize` nem protege cálculo de offset | Valores negativos ou muito grandes podem causar consultas sem limite/overflow. Validar tamanho e página antes de consultar |
| P1 — atualização | `src/routes/peers.rs::update_peer` retorna sucesso sem atualizar registros | Cliente pode acreditar que salvou. Implementar contrato e homologar o endpoint com o cliente |
| P1 — migrações | `src/db.rs` divide SQL por `;`, sem checksum nem transação por migração | Parsing frágil e aplicação parcial. Avaliar migrador SQLx com transição explícita da tabela `_migrations` existente |
| P1 — funcionalidades incompletas | `web/src/views/AuditLogView.vue` chama `GET /api/audit-log`, rota ausente; erros viram lista vazia | A tela pode esconder falhas. Implementar leitura paginada autorizada e distinguir erro de estado vazio |
| P1 — agenda na interface | `web/src/stores/addressBook.ts` carrega apenas a primeira página, padrão 100 | Pesquisa e filtro não enxergam peers de outras páginas. Conectar paginação e filtros ao servidor |
| P2 — desempenho | Loops de consultas em `src/routes/peers.rs` e `src/routes/ab.rs` | Buscar tags e donos em lote; medir antes/depois com a mesma base |
| P2 — concorrência | `ensure_personal_ab` consulta antes de inserir; schema não garante uma agenda pessoal por dono | Requisições simultâneas podem criar duplicatas. Adicionar restrição com saneamento dos dados existentes |
| P2 — tarefas CPU | Hash/verificação Argon2 são executados diretamente em handlers assíncronos | Avaliar `spawn_blocking` e limite de tentativas para não ocupar workers Tokio sob carga |
| P2 — configuração | `token_expiry_hours` consta na configuração, mas a variável de ambiente anunciada no README não é lida por `Config::load` | Alinhar implementação e documentação |
| P2 — interface | Componentes repetem estilos; sidebar fixa; diálogos sem gerenciamento completo de foco | Seguir o guia de layout e extrair componentes por necessidade |

Há bons fundamentos: consultas parametrizadas, constraints de unicidade, separação
por rotas/modelos, hash de senha apropriado e respostas internas sem expor SQL.
O projeto deve ser tratado como uma base inicial, não como produto já homologado.

## Ordem sugerida de evolução

1. Corrigir autorização, revogação de sessões e transações, com testes de regressão.
2. Homologar login, peers, tags, compartilhamento e sincronização com versões reais
   dos clientes; registrar versões, plataforma e evidências.
3. Completar paginação, auditoria e tratamento de falhas na interface.
4. Aplicar o guia de layout gradualmente e medir desempenho.
5. Publicar uma release própria após CI e homologação.

## Limites da revisão

Não foram medidos latência, consumo de memória, concorrência ou compatibilidade
real de protocolo. Não foi feito pentest. As auditorias npm e RustSec foram executadas durante as
correções; ela avalia avisos conhecidos e não prova ausência de vulnerabilidades.
A tabela de achados descreve o upstream; o estado das correções está no início
deste documento. O resultado das verificações locais está registrado no retorno
da tarefa; a CI precisa rodar no repositório de destino.
