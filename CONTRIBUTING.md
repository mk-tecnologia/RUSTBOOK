# Contribuindo

Leia a [análise técnica](docs/ANALISE-TECNICA.md), o [guia de layout](docs/LAYOUT.md)
e as [regras de versionamento](docs/VERSIONAMENTO.md).

## Ambiente e verificação

Usar Node.js 22, npm, Python 3.11+ e Rust stable com Clippy e rustfmt. Executar a
partir da raiz:

```bash
npm --prefix web ci
npm --prefix web test
npm --prefix web run build
python3 scripts/check_version.py
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

O build do frontend precisa preceder a compilação Rust. Para desenvolvimento,
`npm --prefix web run dev` encaminha `/api` para `localhost:21114`; executar o
backend separadamente com `cargo run`. Configurar credenciais em `config.toml`
ou pelas variáveis documentadas em [Configuração e API](docs/REFERENCIA.md). Não versionar segredos ou dados reais.

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

## Padrões de implementação

- Rust: `snake_case` para funções/módulos, `PascalCase` para tipos, formatação por
  rustfmt. Formatar arquivos alterados e evitar reformatação global em PR funcional.
- Vue: arquivos de componentes em `PascalCase.vue`, Composition API e
  `<script setup lang="ts">`. TypeScript: `camelCase` para funções/variáveis.
- Reutilizar tokens e contratos do guia de layout; evitar repetir lógica HTTP.
- Regras de acesso são verificadas no servidor. Visibilidade de botão não autoriza
  uma operação.
- Gravações relacionadas devem usar transação. SQL deve ser parametrizado.
- Migrações novas recebem próximo prefixo numérico; não alterar as já aplicadas.
- Ao corrigir regra de negócio, adicionar regressão que reproduza a falha. Para
  compatibilidade, registrar versão/plataforma do cliente testado.

## Pull requests

Usar branch curta e commits conforme o guia de versionamento. Explicar problema,
mudança, validação executada e limitações; anexar capturas se houver alteração
visual. Atualizar `CHANGELOG.md` para mudanças relevantes ao produto. Builds e
lockfiles devem ser reproduzíveis; não atualizar dependências sem relação com o PR.
