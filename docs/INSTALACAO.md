# Instalação e atualização

## Comando único

Execute os comandos deste guia como **root**. Em uma conta comum, entre com
`su -` antes de continuar. Não é necessário ter `sudo` instalado.

Requisitos no servidor: Linux, acesso root, Docker Engine com Compose v2,
curl e Python 3.11 ou superior. O instalador não instala esses requisitos.
O fluxo padrão baixa uma imagem pronta para a arquitetura do Docker (amd64/arm64);
não precisa de Git, Rust ou Node. Reserve espaço para o download, as imagens nova
e anterior e uma cópia completa do banco.

```bash
curl -fsSL https://raw.githubusercontent.com/mk-tecnologia/RUSTBOOK/main/scripts/install.py | python3 - install latest
```

Repita o mesmo comando para atualizar. `update latest` também é aceito.
`latest` seleciona o build mais recente publicado pela CI após aprovação dos
testes. Os downloads ficam nas pré-releases públicas `build-<commit>` e, quando houver
tag de produto para o commit, na release `vMAJOR.MINOR.PATCH` do GitHub,
com imagens amd64/arm64 e um manifesto SHA-256. Isso não equivale a uma release
homologada. O commit instalado é registrado no servidor. Faça a homologação
com uma cópia do banco e com seus clientes antes de atualizar a produção.

O instalador guarda seus arquivos de controle em `/opt/rustbook`. Em uma
instalação nova, também cria o banco nesse diretório. Ao reconhecer uma agenda
existente, mantém o banco no volume original e usa o Compose original com um
arquivo de sobreposição privado, sem editar o Compose ou o `.env` originais:

| Caminho | Conteúdo |
| --- | --- |
| `.env` | Configuração da instalação nova; cópia do `.env` original em instalações adotadas |
| `data/` | Banco de instalações novas; agendas existentes mantêm o volume original |
| `compose.json` | Compose novo ou sobreposição do Compose original |
| `install-state.json` | Versão, imagem e contexto da implantação, incluindo configuração privada |
| `backups/` | Cópias consistentes dos dados e da configuração |
| `manage.sh` | Executa o Compose correto para esta implantação |

Quando não existe agenda, a primeira execução cria credenciais aleatórias e um banco novo. Nas próximas,
a configuração e os usuários são preservados. A senha do `.env` só cria o admin
quando o banco está vazio; ela não redefine senhas de contas existentes.

A imagem é baixada, verificada por SHA-256 e carregada no Docker antes da interrupção. Depois, a agenda é parada, os dados
são copiados e a nova imagem é iniciada. A API precisa responder com a versão
esperada em até 60 segundos. Em falha, o instalador tenta restaurar o banco e a
imagem anteriores e retorna erro. Se essa restauração também falhar, mantenha o
serviço parado e use o backup informado na saída. Confirme os logs e o acesso
pelo cliente após cada atualização.

Backups e imagens anteriores não são apagados automaticamente. Monitore o espaço
em disco e mantenha cópias externas: um backup no mesmo disco não protege contra
a perda do servidor. O instalador não configura TLS, firewall, hbbs ou hbbr.

## Atualização rápida e compilação opcional

Se o commit publicado já estiver instalado, o instalador confere o container, o
ID da imagem e a API e termina sem download da imagem, backup ou reinício.
Se ainda não houver um build publicado, ele informa que é preciso aguardar a CI;
não começa uma compilação demorada automaticamente. Um commit novo na `main`
pode levar alguns minutos para aparecer nos downloads, enquanto a CI trabalha.

Para compilar a `main` diretamente no servidor, escolha explicitamente `--build`:

```bash
curl -fsSL https://raw.githubusercontent.com/mk-tecnologia/RUSTBOOK/main/scripts/install.py | python3 - install latest --build
```

Esse modo adicional exige Git e recursos para compilar. O Dockerfile mantém as
dependências Rust em uma camada independente dos fontes e do frontend, para que
alterações de interface recompilem apenas o aplicativo. Mudanças nos manifests,
lockfiles ou imagens base podem exigir recompilação das dependências. A CI também
reaproveita essas camadas com cache separado por arquitetura.

## Acesso e configuração

Em instalações reconhecidas, o IP e a porta existentes são preservados.
As orientações de `BIND_IP` abaixo se aplicam a instalações novas.

O endereço padrão de uma instalação nova é `http://127.0.0.1:21114`. Configure um proxy HTTPS no host
para encaminhar à porta 21114. Proxies em outro container precisam de configuração
de rede própria; o loopback desse container não é o loopback do host.

Consulte as credenciais como root:

```bash
cat /opt/rustbook/.env
```

Para mudar o IP de escuta, edite `BIND_IP` nesse arquivo. Use o IP interno do
servidor para acesso pela LAN ou `0.0.0.0` para todas as interfaces, com firewall
e TLS apropriados. Preserve `RUSTDESK_AB_JWT_SECRET`. Outras opções estão na
[referência de configuração](REFERENCIA.md).

Depois de editar a configuração:

```bash
/opt/rustbook/manage.sh up -d --no-deps address-book
```

Logs:

```bash
/opt/rustbook/manage.sh logs --tail=100 address-book
```

Para instalações novas, mantenha a porta interna em 21114, o caminho do banco em `/data/db.sqlite3` e os
arquivos gerenciados no mesmo diretório. Edite a configuração no `.env`; o
`compose.json` é regenerado pelo instalador. Variáveis exportadas no seu terminal
não substituem a configuração persistente do `.env` para o container.

## Migrar uma agenda existente

Use o mesmo comando `install latest`. Não pare nem renomeie o container antes:
o instalador faz a parada somente depois de baixar e verificar a nova imagem.

A detecção automática atende o cenário:

- Container chamado `rustdesk-address-book`, serviço Compose `address-book`.
- Arquivos Compose e diretório de trabalho originais presentes no host.
- Volume bind gravável em `/data`, contendo `db.sqlite3` e sem links simbólicos.
- `RUSTDESK_AB_DB_PATH=/data/db.sqlite3` e chave JWT persistente de pelo menos
  32 bytes no ambiente do container.
- Porta HTTP publicada no host para verificar a versão após a atualização.

Por exemplo, uma agenda com Compose em
`/opt/rustdesk-address-book/compose.deploy.yaml` e banco em
`/opt/rustdesk-address-book/data` continua usando exatamente esses caminhos.
O instalador mantém o nome do projeto Compose e as portas, redes, volumes e
configuração definidos ali. Somente a imagem da agenda e as variáveis
`RUSTDESK_AB_*` efetivamente usadas pelo container são fixadas na sobreposição.
Os demais serviços não são parados nem recriados.

O backup inclui banco, WAL/SHM, configuração original e identificação da imagem
anterior. Em falha, a imagem anterior é recuperada pelo ID local, o banco e as
permissões originais são restaurados. Não é necessário que a versão antiga tenha
`/api/version`: nesse caso o rollback verifica se o container permanece em execução.
Os dados da tentativa que falhou ficam em um diretório `data.failed-*` ao lado do
volume original, para diagnóstico. Sessões antigas podem exigir novo login.

Após a adoção, use o comando único para atualizar e `/opt/rustbook/manage.sh`
para operações de Compose. Executar apenas o Compose original pode recriar a
imagem antiga. Não apague os arquivos originais: o instalador continua usando-os.
As variáveis em uso são preservadas na sobreposição; editar apenas o `.env` original
não substitui essas variáveis fixadas. Para alterar uma delas, edite o campo
`environment` no `/opt/rustbook/compose.json` privado e aplique com `manage.sh up`.
Valores literais com `$` devem usar `$$` nesse JSON. A próxima atualização preserva
os valores efetivamente aplicados ao container.

Se algum requisito não for atendido, o instalador interrompe antes de parar a
agenda e informa a incompatibilidade. Não cria outro banco para contornar a falha.

### Importação offline alternativa

Para instalações que não possam ser reconhecidas por Compose, permanece disponível
`install latest --import-from /caminho/da/copia`. Prepare uma cópia consistente com
`data/db.sqlite3`, WAL/SHM e `.env` contendo a chave original em
`RUSTDESK_AB_JWT_SECRET`. Pare a agenda de origem antes de copiar; libere o nome
`rustdesk-address-book` renomeando o container antigo parado. Esse modo copia os
dados para `/opt/rustbook`, mantém a origem e exige conferir manualmente portas e
configuração. Ele não conhece a imagem antiga para rollback automático inicial.
Prefira a detecção automática quando os metadados Compose estiverem disponíveis.

## Instalação manual

O `compose.deploy.example.yaml` permanece disponível para quem administra seu
próprio Compose. Veja [implantação isolada](COMPATIBILIDADE-RUSTDESK.md).
O `docker-compose.yml` da raiz inclui hbbs/hbbr e destina-se à stack completa.
Esses métodos têm configuração e procedimento de atualização próprios; o comando
único reconhece automaticamente o cenário de Compose descrito acima, além de
instalações novas e importações offline.
