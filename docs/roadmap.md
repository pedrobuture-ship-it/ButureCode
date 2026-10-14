# Roadmap

Pedro Buture ampliou o ciclo até o prompt 50 e depois pediu uma pausa com retomada no prompt 45. Prompts 01–05 já estavam concluídos. O avanço mantém commits por etapa.

1. Bootstrap e arquitetura de workspace.
2. Regras de autoria e desenvolvimento.
3. Tokens Buture Glass Dark.
4. GlassSurface e Full / Reduced / Off.
5. Janela nativa.
6. Top bar e Command Center visual.
7. Activity navigation e seleção contextual.
8. Sidebar redimensionável e árvore virtualizada.
9. Explorer real, preview somente leitura, operações de arquivos e watcher por polling.
10. Settings Default/User/Workspace e persistência INI validada.
11. Detecção somente leitura do host.
12. Onboarding com seleção de linguagens e conclusão persistida.
13. Arquitetura extensível de providers e sondagens limitadas.
14. Rust e instalações rustup adicionais.
15. C/C++, Clang, GCC, caminhos MSVC e ferramentas de build.
16. Python e diretórios conhecidos de ambientes.
17. Java, JDK, Maven e Gradle.
18. Node.js, gerenciadores de pacotes e indícios de version managers.
19. Settings → Development → Toolchains com resultados observados.
20. Instalação assistida: revisão do comando, confirmação, execução e validação.

21. Motor Buture Doctor com diagnósticos tipados e evidências.
22. Workspace visual do Doctor e ações revisadas.
23. Detecção automática de projetos e monorepos.
24. Project Overview com manifests e estados reais.
25. Buffer Rope UTF-8, transações, undo/redo e salvamento.
26. Editor nativo com renderização de linhas visíveis.
27. Tema de sintaxe Buture Emerald centralizado.
28. Tabs, pins, preview e persistência de layout.
29. Split editor com buffers compartilhados e views independentes.
30. Parsing incremental Tree-sitter e folding.
31. Operações avançadas de edição.
32. Busca e substituição em documento/workspace.
33. Command Palette e registro de comandos.
34. Quick Open fuzzy com índice sob demanda.
35. Keybindings por SO, overrides, conflitos e chords.

36. Terminal PTY, tabs, split, ANSI e entrada/saída reais.
37. Task runner com dependências, revisão e execução em worker.
38. Smart Run com detecção de candidatos e escolha explícita.
39. Cliente LSP stdio, lifecycle e isolamento do loop da UI.
40. Completion, hover, navegação, rename, symbols e code actions revisadas.
41. Registry de formatters e format on save habilitado pelo usuário.
42. Problems Panel com diagnósticos recebidos e navegação.
43. Domínio Git nativo com planos de operações e identidade configurada.
44. Source Control com Changes/Staged Changes, branches e revisão de commit.

Etapas 01–44 implementadas, com limites nos [marcos 35](milestone-35.md) e [44](milestone-44.md). O prompt 45 foi iniciado, mas permanece pendente de revisão e validação: seu patch está preservado fora da árvore de código ativa.

Próximo: 45 (diff inline/side-by-side, stage/unstage hunk e discard confirmado). Depois, 46 Git Graph, 47 Commit Assistant, 48 Debugger Core, 49 Debugger UI e 50 Memory Inspector. Não antecipar 51–95. A instrução mais recente é parar agora e retomar no 45; consultar [o arquivo de retomada](../RETOMADA-PROMPT-45.md).
