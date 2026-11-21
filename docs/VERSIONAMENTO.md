# Versionamento e GitHub

## Versão do produto

A versão inicial herdada é `0.1.0`. `0.2.0` foi usada em commits de
desenvolvimento, sem tag/release. A primeira release de produto desta continuação
é `0.3.0`, com imagens prontas e identificação do build. A versão `0.4.0`
adiciona pastas de dispositivos e associação de tags no console. Backend e frontend são distribuídos juntos e usam a mesma
versão, apesar de o pacote web ser privado.

- `Cargo.toml` é a fonte da versão do produto.
- A entrada do próprio projeto em `Cargo.lock`, `web/package.json` e as duas
  entradas de versão raiz em `web/package-lock.json` devem acompanhar essa versão.
- `python3 scripts/check_version.py` verifica consistência sem instalar pacotes.
- Tags de produto seguem `vMAJOR.MINOR.PATCH`, por exemplo `v0.3.0`.
- `GET /api/version` retorna a versão do manifesto e o SHA compilado no binário.
  O cabeçalho mostra `v0.3.0 · abc1234`; consultar a versão não depende do cache
  do frontend. O SHA é injetado no build Docker e não por uma variável de runtime.

Usar [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html). Depois de 1.0,
patch corrige sem quebra, minor adiciona funcionalidade compatível e major muda o
contrato de forma incompatível. Antes de 1.0, adotamos: patch para correções
compatíveis e minor para funcionalidades ou quebras, sempre anunciando as quebras.
API pública inclui endpoints e payloads usados pelo RustDesk, configuração e
procedimento de migração do banco. Não renomear endpoints para adicionar `/v1`
sem preservar o protocolo dos clientes existentes.

## Branches e commits

- `main`: integração das mudanças revisadas e aprovadas pela CI.
- Branches curtas: `feat/paginacao-peers`, `fix/permissao-agenda`, `docs/layout`,
  `chore/ci`. Uma intenção por PR.
- Preferir squash merge, título do PR como mensagem final.
- [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/):
  `feat(peers): adicionar paginação`, `fix(auth): revogar sessões`,
  `docs: documentar instalação`, `ci: verificar versões`.
- Usar `!` e rodapé `BREAKING CHANGE:` quando houver quebra; explicar migração.
- Não reescrever tags publicadas ou histórico compartilhado.

## CI

O workflow `CI` valida versões, audita dependências npm/Rust, faz instalação pelo lockfile e build completo da
interface, e executa testes e Clippy no Rust. O build web ocorre antes do Rust
porque `rust-embed` exige `web/dist/` durante a compilação.

O job `quality` usa Node 22 e toolchain Rust stable, além de verificar rustfmt.
A validação local usou Rust 1.100.0; uma MSRV anterior não foi homologada. A CI não prova
compatibilidade com todas as versões dos clientes ou capacidade em produção.
`cargo test` executa as regressões de permissões, sessões, transações, paginação
e migrações adicionadas nesta continuação.

## Procedimento de release

1. Abrir PR de release e escolher a versão conforme o impacto.
2. Alterar `version` em `Cargo.toml` e executar `cargo check` para atualizar a
   entrada do projeto em `Cargo.lock` (com `web/dist/` já compilado).
3. Em `web/`, executar `npm version 0.2.1 --no-git-tag-version`, substituindo o
   exemplo pela versão escolhida. Isso atualiza package e lockfile juntos.
4. Registrar as mudanças de `Unreleased` em `CHANGELOG.md` sob a versão e a data
   real; informar migrações, backup e quebras. Não inventar releases do upstream.
5. Executar `python3 scripts/check_version.py`, build web, `cargo test --locked`
   e `cargo clippy --locked --all-targets -- -D warnings`. Homologar com cliente RustDesk.
6. Integrar o PR com CI aprovada. No commit integrado, criar uma tag anotada, por
   exemplo `git tag -a v0.2.1 -m 'Release v0.2.1'`.
7. Publicar o commit na `main` e sua tag de produto juntos. O workflow verifica
   que tags `v*` correspondem aos manifests. Após a CI da `main`, o job de publicação
   cria a release da versão quando essa tag aponta para o commit compilado, com
   notas do changelog e os mesmos assets amd64/arm64 e manifesto.
   Se a tag for publicada depois, reexecute a CI da `main` no commit correspondente.

A escolha da versão e a criação das tags de produto (`v*`) seguem o procedimento
acima; a publicação dos assets e da release é automática após aprovação da CI.
Após a CI da `main`, jobs nativos amd64/arm64 publicam pré-releases automáticas
`build-<commit>` com imagens Docker comprimidas e manifesto SHA-256. O instalador
usa o build publicado mais recente; isso não declara homologação de produto.
Os assets de um build publicado são preservados em reexecuções. Os templates de
Compose manual mantêm a opção de build local. hbbs/hbbr usam suas imagens upstream.

## Configuração do repositório de destino

As regras a seguir precisam ser habilitadas no GitHub; arquivos locais não
configuram proteções de branch:

- Repositório próprio público: [mk-tecnologia/RUSTBOOK](https://github.com/mk-tecnologia/RUSTBOOK).
  O `origin` aponta para ele; `upstream` preserva `ds4a/rustdesk-address-book`.
  Publique alterações próprias em `origin` e preserve o histórico original.
- No repositório próprio, habilitar Actions, proteger `main`, exigir PR e check
  `quality` do workflow `CI`; impedir force push e exclusão.
- Exigir revisão de outro mantenedor quando houver equipe disponível. Em projeto
  individual, manter PR e CI sem criar uma regra impossível de autoaprovação.
- Usar squash merge e excluir branches integradas.
- Manter a atribuição ao upstream e a licença AGPL-3.0 declarada no manifesto.
  O arquivo `LICENSE` desta continuação contém o texto da licença.

Referência: [GitHub — protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches).
