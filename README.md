# RUSTBOOK

Agenda de dispositivos e painel web para **RustDesk OSS**, com usuários, grupos,
tags e auditoria. O painel oferece temas claro/escuro e exibe a versão instalada.

Este projeto é um **clone com alterações próprias** de
[ds4a/rustdesk-address-book](https://github.com/ds4a/rustdesk-address-book),
baseado no commit `ca39fa3`, mantido por
[mk-tecnologia](https://github.com/mk-tecnologia). O histórico e os créditos dos
autores originais foram preservados. Licença [AGPL-3.0](LICENSE).

## Instalar ou atualizar

Em um servidor **Linux**, com **Docker Engine, Docker Compose v2, curl e
Python 3.11+** instalados, execute **como root**:

```bash
curl -fsSL https://raw.githubusercontent.com/mk-tecnologia/RUSTBOOK/main/scripts/install.py | python3 - install latest
```

Se estiver usando uma conta comum, entre como root com `su -` antes de executar.

**O mesmo comando instala e atualiza.** Ele baixa a imagem pronta mais recente
publicada pela CI para **amd64 ou arm64**, verifica sua integridade e aplica a
atualização. **Não compila no servidor.** Se o mesmo commit já estiver em execução
e saudável, termina sem baixar a imagem ou reiniciar. Versão atual: **[0.5.0](https://github.com/mk-tecnologia/RUSTBOOK/releases/tag/v0.5.0)**.

Para configurar Online/Offline pelo servidor de ID, veja [Presença dos dispositivos](docs/PRESENCA.md). Relatórios antigos da API não significam que o peer está offline.

O instalador:

- Instala somente a agenda; reconhece instalações Compose existentes e preserva
  o volume, as portas, as credenciais, `hbbs` e `hbbr`.
- Gera as credenciais na primeira instalação e as preserva nas próximas.
- Baixa e verifica a imagem antes de parar a agenda para atualizar.
- Faz backup do banco e da configuração com a agenda parada.
- Inicia a nova versão, verifica a resposta da API e tenta restaurar a versão e o
  banco anteriores se a atualização falhar.

**Já tem uma agenda no container `rustdesk-address-book`?** O mesmo comando
reconhece a instalação Compose e atualiza no diretório existente, sem importação
manual. Veja os [requisitos de reconhecimento](docs/INSTALACAO.md#migrar-uma-agenda-existente).
Homologue a atualização com seu cliente RustDesk antes de aplicá-la em produção.

## Organizar dispositivos

Na página **Devices**, use **New group** para criar pastas por cliente ou local.
Em **Add device**, escolha o grupo e marque uma ou mais tags; também é possível
criar tags dentro do formulário. Clique no nome ou no lápis de um dispositivo
para movê-lo de grupo ou alterar suas tags.

Os grupos expandem para mostrar os dispositivos, com busca, filtro por tag e
paginação independente. Excluir um grupo mantém seus dispositivos em **Ungrouped**;
excluir uma tag mantém os dispositivos. **User groups** continuam separados para
controle de acesso. Veja o [guia de grupos e tags](docs/GRUPOS-E-TAGS.md).

## Primeiro acesso

Em uma instalação nova, o serviço fica em `http://127.0.0.1:21114`, acessível no próprio servidor.
Publique esse endereço por um proxy com HTTPS. O usuário inicial é `admin`;
consulte a senha gerada no servidor:

```bash
cat /opt/rustbook/.env
```

Em instalações existentes, o acesso e as credenciais atuais são mantidos.
Esse arquivo também pode conter a chave JWT: mantenha-o privado. Para acesso direto
pela rede interna, consulte [acesso e configuração](docs/INSTALACAO.md#acesso-e-configuração).

No cliente RustDesk, configure **API Server** com a URL HTTPS da agenda. Mantenha
**ID Server**, **Relay Server** e a chave do seu `hbbs` com os valores existentes.
RUSTBOOK cuida da agenda; `hbbs`/`hbbr` cuidam das conexões remotas.

## Documentação

- [Instalação, migração e comandos de operação](docs/INSTALACAO.md)
- [Backup, rollback e cuidados em produção](docs/OPERACAO.md)
- [Configuração e referência da API](docs/REFERENCIA.md)
- [Compatibilidade com o cliente RustDesk](docs/COMPATIBILIDADE-RUSTDESK.md)
- [Desenvolvimento e contribuição](CONTRIBUTING.md)
- [Histórico de alterações](CHANGELOG.md)

A criação de compartilhamentos e a edição de membros dos grupos de usuários
ainda não estão completas no painel. As pastas de dispositivos são organização
da agenda, sem conceder permissões adicionais. Grupos e ACLs do inventário nativo
continuam pendentes. Consulte os limites no guia de compatibilidade.
