# RUSTBOOK

Agenda de dispositivos e painel web para **RustDesk OSS**, com usuários, grupos,
tags e auditoria. O painel oferece temas claro/escuro e exibe a versão instalada.

Este projeto é um **clone com alterações próprias** de
[ds4a/rustdesk-address-book](https://github.com/ds4a/rustdesk-address-book),
baseado no commit `ca39fa3`, mantido por
[mk-tecnologia](https://github.com/mk-tecnologia). O histórico e os créditos dos
autores originais foram preservados. Licença [AGPL-3.0](LICENSE).

## Instalar ou atualizar

Em um servidor **Linux**, com **Docker Engine, Docker Compose v2, Git, curl e
Python 3.11+** instalados, execute:

```bash
curl -fsSL https://raw.githubusercontent.com/mk-tecnologia/RUSTBOOK/main/scripts/install.py | sudo python3 - install latest
```

**O mesmo comando instala e atualiza.** `latest` significa o código mais recente
da branch `main`. A imagem é compilada no servidor; a primeira instalação pode
demorar vários minutos. Versão atual do produto: **0.2.0**.

O instalador:

- Instala somente a agenda em `/opt/rustbook`, preservando `hbbs` e `hbbr`.
- Gera as credenciais na primeira instalação e as preserva nas próximas.
- Constrói a imagem antes de parar a agenda para atualizar.
- Faz backup do banco e da configuração com a agenda parada.
- Inicia a nova versão, verifica a resposta da API e tenta restaurar a versão e o
  banco anteriores se a atualização falhar.

**Já tem uma agenda instalada por outro método?** Faça a
[migração inicial](docs/INSTALACAO.md#migrar-uma-agenda-existente) antes de usar o
comando. O instalador recusa substituir um container ou dados que não gerencia.
Homologue a atualização com seu cliente RustDesk antes de aplicá-la em produção.

## Primeiro acesso

Por padrão, o serviço fica em `http://127.0.0.1:21114`, acessível no próprio servidor.
Publique esse endereço por um proxy com HTTPS. O usuário inicial é `admin`;
consulte a senha gerada no servidor:

```bash
sudo cat /opt/rustbook/.env
```

Esse arquivo também contém a chave JWT: mantenha-o privado. Para acesso direto
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

A criação de compartilhamentos e a edição de membros de grupos ainda não estão
completas no painel. O inventário do cliente é básico; grupos e permissões por
dispositivo continuam pendentes. Consulte os limites no guia de compatibilidade.
