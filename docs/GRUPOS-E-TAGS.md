# Grupos de dispositivos e tags

A página **Devices** organiza os peers da agenda pessoal em pastas expansíveis,
como grupos por cliente, filial ou local. Cada dispositivo pertence a **um grupo
por vez** e pode receber **várias tags**. Uma mesma tag pode ser usada em vários
grupos, por exemplo `Production`, `Windows` ou `Finance`.

## Criar e organizar

1. Use **New group**, informe o nome e, opcionalmente, uma descrição.
2. Use **Add device** na barra superior ou o botão `+` na linha de um grupo.
3. Informe o RustDesk ID, nome e plataforma, escolha o grupo e marque as tags.
4. Para criar uma tag durante o cadastro, informe nome e cor no campo **New tag**
   e clique em **Add**. A tag é criada na agenda e selecionada no formulário.
5. Salve o dispositivo. A linha do grupo mostra a contagem atualizada.

Para mover um peer existente ou alterar suas tags, clique no nome ou no lápis da
linha. O ID permanece fixo durante a edição. Escolha outra pasta ou **Ungrouped**,
marque/desmarque tags e salve. O cadastro de um ID já existente pelo botão de
inclusão retorna conflito; ele não substitui silenciosamente o registro anterior.

As tags também podem ser criadas pelo `+` da lateral. Criar uma tag não exige
salvar o formulário de dispositivo: se o formulário for cancelado depois, a tag
criada permanece disponível na agenda.

## Busca e paginação

A busca consulta ID, nome, hostname e username no servidor. O filtro por tag
combina com a busca e atualiza as contagens de cada pasta. Pastas sem resultados
ficam ocultas durante os filtros. Cada grupo aberto carrega sua própria página
de até 24 dispositivos; **Next/Previous** navegam apenas dentro daquela pasta.
As contagens refletem todos os resultados, não somente os peers carregados.

## Exclusão e segurança

- Excluir um grupo mantém todos os seus peers em **Ungrouped**, com as tags.
- Excluir uma tag remove apenas essa associação; não apaga peers nem suas pastas.
- Excluir um peer o remove da agenda; não desinstala nem altera o RustDesk remoto.
- Grupos e tags são isolados por agenda e seguem suas permissões de leitura/escrita.
  Não criam permissões de acesso remoto nem dão acesso a outras agendas.
- **User groups** são os grupos administrativos de usuários existentes e continuam
  separados. Não são convertidos nem excluídos por esta funcionalidade.
- Online/offline é uma estimativa a partir dos relatos recebidos recentemente.
  Está disponível apenas para admins com acesso ao inventário. Sem relato ou
  autorização, o painel mostra **Unknown**, sem inventar um estado online.

## Compatibilidade e atualização

A versão 0.4.0 adiciona a migração `009_peer_groups.sql`: cria as pastas e adiciona
`group_id` opcional aos peers. Os dispositivos existentes ficam em **Ungrouped**;
as tags e grupos de usuários existentes são mantidos. Faça backup antes de atualizar.
O instalador faz a cópia offline e pode restaurar o banco e a imagem anteriores.

Os endpoints antigos de peers e tags permanecem disponíveis. Atualizações que não
informam `group_id` preservam a pasta atual; enviar `group_id: null` remove a
associação. A sincronização legada de toda a agenda preserva os grupos dos IDs que
continuam presentes. Pastas vazias não são apagadas por essa sincronização.

Estas pastas pertencem à agenda do console. Não implementam grupos/ACLs do inventário
nativo da aba Dispositivos do cliente RustDesk. Essa API continua com suas restrições
anteriores. As tags mantêm o contrato existente do cliente. Confira o
[guia de compatibilidade](COMPATIBILIDADE-RUSTDESK.md) e homologue seus clientes.

## Conectar pelo painel

Na lista de dispositivos, clique no ícone azul de conexão nas ações da linha.
O navegador solicita abrir o RustDesk instalado neste computador e envia o ID
pelo protocolo `rustdesk://connect/ID`. O cliente usa os servidores já configurados
e solicita a senha remota ou a aprovação de acesso conforme a configuração do peer.
A conexão acontece no aplicativo nativo. É necessário ter RustDesk instalado e
associado ao protocolo; o navegador pode pedir confirmação para abri-lo.
O link não carrega senha nem token da agenda. Quando configurada, inclui a chave pública do servidor. Presença desconhecida/offline
não bloqueia a tentativa de conexão. Para editar o dispositivo, use o lápis.

### Conexões com hbbs OSS e conta nativa logada (0.6.1+)

Na versão 0.7.0+, use **Server settings** (nas versões 0.6.x, **Devices → Connection settings**) e informe:

- **ID server · TCP endpoint:** endereço do hbbs acessível pelos computadores dos
  operadores, com a porta de ID (normalmente `rustdesk.example.com:21116`). Esse
  endereço pode ser diferente do endereço interno usado para consultar presença.
- **Public Key:** a chave pública do hbbs já configurada no campo Key dos clientes.
  Não informe a chave privada, a senha do computador nem o token da conta.

Salve. Os ícones de conexão passam a gerar links no formato
`rustdesk://connect/ID@servidor:porta?key=CHAVE_PUBLICA`, sem IP fixo no código.
O cliente usa a conexão explícita a esse servidor, preservando sua chave pública
sem encaminhar o token da conta da agenda ao hbbs OSS. A senha remota ou a aprovação
continua necessária. A configuração fica no banco e é preservada nas atualizações.
Limpar os dois campos restaura os links simples que usam o padrão do cliente.

Essa correção aplica-se aos atalhos do painel. Para conexões iniciadas pelo ID
simples no aplicativo RustDesk 1.5.3 com hbbs OSS, sair da conta em
**Configurações → Conta → Logout** evita o fluxo de handshake com o token da agenda.
Outra opção no cliente é informar `ID@servidor:porta?key=CHAVE_PUBLICA` no campo de
conexão. Não remova a Key para contornar o erro e não altere a senha do peer.

## Layout e pesquisa de tags (0.6.2+)

Os boxes de presença, conexões e filtro de tags compartilham uma grade responsiva;
em telas estreitas são empilhados. A lista usa a largura disponível, com colunas
alinhadas entre grupos e rolagem horizontal em telas pequenas.

Abra o seletor de tags e digite para pesquisar. A busca ignora acentos e diferenças
de maiúsculas/minúsculas. No cadastro/edição, várias tags podem ser selecionadas;
a busca não altera as seleções anteriores. Use as setas e Enter para escolher,
Escape para fechar a lista e Done para concluir a multisseleção. A lista tem altura
limitada e rolagem própria; novas tags continuam podendo ser criadas no formulário.
Para excluir uma tag, selecione-a no filtro e use o ícone de lixeira do box Tags.

As linhas mostram até duas tags e um contador das demais; o tooltip e a edição
permitem consultar os nomes completos. Presença Offline reflete a resposta do hbbs
para o ID cadastrado; confira todos os dígitos se outro ID conecta no cliente.

## Configuração central (0.7.0+)

Os boxes da agenda agora levam a **Server settings** e **Client setup**. O primeiro
reúne presença e configuração de conexões, além do relay público anunciado pelo
hbbs. O segundo mostra os quatro campos prontos para o cliente RustDesk. A agenda
continua usando a mesma configuração de presença e a mesma Key já salvas. Veja
[gerenciamento do servidor](CONFIGURACAO-SERVIDOR.md).
