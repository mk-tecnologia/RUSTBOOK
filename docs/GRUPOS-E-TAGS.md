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
