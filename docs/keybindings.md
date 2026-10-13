# Atalhos — prompt 35

Registro de comandos e keybinding engine vivem em `buture-core`, sem GUI. Defaults usam Cmd no macOS e Ctrl no Windows/Linux. Escopos Global/Editor evitam ações de edição quando um campo de busca ou Settings está focado. Palette, Quick Open e diálogos tratam sua própria navegação.

Settings → Keyboard Shortcuts edita o perfil pessoal `keybindings.ini`, junto às preferências do usuário. Arquivos do workspace não mudam atalhos. Save valida antes de escrever; Reload detecta mudanças externas. Perfil inválido não substitui o mapa ativo, e arquivo externo não é sobrescrito sem recarregar. Perfil ausente usa defaults. Limite 64 KiB.

```ini
# Primary = Cmd no macOS, Ctrl nos demais
editor.save = Primary+Alt+S
editor.comment = none
commands.open = Primary+Shift+P, Primary+K
```

Exemplo alternativo de chord:

```ini
# Desabilitar o default simples libera o prefixo para um chord
commands.open = Primary+Shift+P
editor.save = Primary+K S
```

Os exemplos são alternativas: um perfil não pode repetir o mesmo command.id. Cada override substitui todos os defaults daquele comando; `none` desabilita. Alias separado por vírgula, sequência de até duas teclas separada por espaço. Conflitos idênticos e conflitos entre uma tecla simples e prefixo de chord são rejeitados quando os escopos se sobrepõem. Chord expira após 1,5 s; a tecla consumida não entra no documento. IDs desconhecidos e teclas inválidas geram erro.

| Ação | Default Primary |
| --- | --- |
| Open Folder / File | O / Shift+O |
| Quick Open / Palette | P / Shift+P ou K |
| Save / Undo / Redo | S / Z / Shift+Z (Ctrl+Y adicional fora do macOS) |
| Find / Find workspace | F / Shift+F |
| Close / Restore tab | W / Shift+T |
| Split right / down | Backslash / Shift+Backslash |
| Next occurrence | D |
| Comment | Slash |

A tela lista os atalhos ativos. Defaults por plataforma têm testes unitários; execução nativa foi validada apenas no Linux. Ctrl/Cmd não instala ferramentas nem executa comandos definidos por projetos.
