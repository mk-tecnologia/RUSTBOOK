# Padrão de layout e componentes

Este guia define a evolução da interface existente em Vue + Tailwind. Não implica
que todas as telas atuais já atendam ao padrão.

## Estrutura do código

```text
src/
  auth/       # credenciais, tokens e extração de identidade
  models/     # entidades e contratos serializáveis
  routes/     # adaptação HTTP; regras reutilizáveis devem sair dos handlers
  db.rs       # pool e aplicação de migrações
web/src/
  api/        # cliente HTTP e contratos tipados
  stores/     # estado compartilhado e ações
  views/      # composição das páginas
  components/
    layout/   # estrutura de navegação
    peers/    # componentes do domínio de dispositivos
    tags/     # componentes do domínio de tags
  style.css   # tokens, tema e estilos globais
migrations/   # SQL sequencial; nunca editar migração aplicada
```

Criar `services/` no backend quando houver regra reutilizável ou transação que
precise ser compartilhada. Criar `components/ui/` quando extrair botões, campos,
modais e estados reutilizáveis. Evitar camadas sem uma necessidade concreta.

## Composição de página

- `AppLayout` é responsável por navegação, cabeçalho e área principal.
- Cada página tem um `h1`, descrição opcional, ação principal e conteúdo.
- Toolbar agrupa pesquisa, filtros e ações; deve quebrar linha em telas estreitas.
- Desktop: sidebar de 240 px e cabeçalho de 56 px; conteúdo com padding de 24 px.
- Abaixo de 768 px: navegação recolhível com botão acessível, padding de 16 px e
  filtros organizados acima da lista. A navegação já pode ser recolhida em telas móveis.
- Cards usam grid responsivo. Tabelas usam rolagem horizontal localizada; a página
  inteira não deve transbordar.
- Validar em 360, 768 e 1440 px, nos temas claro e escuro, e com zoom de 200%.

## Tokens visuais

Usar os tokens existentes de `web/src/style.css` em vez de novas cores isoladas.

| Uso | Regra |
| --- | --- |
| Ação principal | `rd-primary`, hover `rd-primary-hover` |
| Fundo escuro | `rd-scaffold`; superfícies `rd-card`; bordas `rd-border` |
| Texto escuro | `rd-text` e `rd-text-secondary` |
| Tema claro | Fundo `gray-50`, superfícies brancas, texto `gray-900`, bordas `gray-200` |
| Estados | `rd-success`, `rd-warning`, `rd-danger`, sempre acompanhados de texto/ícone |
| Espaçamento | Escala de 4 px: 4, 8, 12, 16, 24, 32 |
| Tipografia | Fonte de sistema; texto 14–16 px; metadados 12 px; título de página 20–24 px |
| Bordas | Cards 12 px; controles 8 px; tags 4 px |
| Foco | Indicador visível por teclado; nunca remover outline sem substituição |

O seletor de tema deve respeitar a escolha persistida e, sem escolha, a preferência
do sistema. A preferência persistida é `system`, `light` ou `dark`; nunca salvar o
resultado atual do sistema como uma escolha manual. Mudanças de preferência do
sistema são acompanhadas enquanto o modo automático está ativo. Escolhas manuais
têm prioridade e também atualizam o color-scheme dos controles nativos.

O cabeçalho consulta `GET /api/version` sem cache para exibir a versão do servidor.
Quando a API está indisponível ou o backend é antigo, mostrar "Version unavailable"
em vez de inferir a versão instalada pelo pacote web. Essa consulta é pública e
não retorna credenciais, caminhos ou configuração.

No Tailwind 4, definir `@custom-variant dark` para que as classes
`dark:*` acompanhem a classe `.dark` usada pelo store.

## Contratos de componentes

- Botão: variantes principal, secundária e destrutiva; disabled e loading explícitos.
- Campo: label associado por `for/id`, ajuda opcional e mensagem de validação;
  placeholder não substitui label.
- Modal: nome acessível, `role="dialog"`, `aria-modal="true"`, foco inicial,
  contenção de foco, Escape e restauração do foco ao fechar.
- Lista/tabela: estados de carregamento, vazio, erro com nova tentativa e sucesso;
  vazio não deve esconder erro de rede.
- Ações de exclusão: confirmação com nome do registro, botão destrutivo e prevenção
  de submissão repetida.
- Ícones decorativos: `aria-hidden="true"`; botão apenas com ícone tem nome acessível.
- Links navegam; botões executam ações. Não depender só de cor para indicar estado.

Componentes recebem props e emitem eventos tipados. Chamadas HTTP ficam em `api/`;
estado compartilhado fica nos stores. Evitar `any` nos contratos novos. Manter
classes Tailwind completas no código; concatenações como `color + '/10'` não são
um padrão confiável de geração de CSS.

## Revisão visual em PR

Registrar capturas antes/depois para mudanças visuais, viewport, tema, navegação
por teclado, mensagens de erro e estado vazio. Mudanças de idioma devem seguir uma
decisão explícita de internacionalização; não misturar traduções isoladas.

Referência: [Tailwind — dark mode](https://tailwindcss.com/docs/dark-mode).
