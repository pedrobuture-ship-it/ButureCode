# Terminal — prompt 36

Terminal na top bar e Command Palette alterna o painel. New shell inicia explicitamente o shell absoluto do ambiente (SHELL/COMSPEC, fallback Unix) em um PTY real, no workspace ou home. Sessões independentes, rename, split, kill, resize, alternate screen ANSI, cores indexadas/truecolor e até 2000 linhas de scrollback. Texto em superfície sólida; glass permanece nas regiões do shell.

IO e parsing fora da UI, filas limitadas, snapshots até 30 Hz e dimensões até 200×400. Clipboard: Paste, Copy screen, Ctrl+C para interrupção. Cursor/teclado básicos; scroll com wheel. Colagem limitada a 1 MiB, com bracketed paste quando solicitado pelo shell. O caminho exibido é o cwd inicial; não há detecção portátil de cada `cd` interno. Não há mouse reporting, seleção retangular, ligaduras ou promessa de fidelidade completa a todos os programas fullscreen.

A sessão roda com permissões do usuário; startup files do shell são código do ambiente pessoal. Nunca iniciar shell ao simplesmente abrir um projeto. Fechamento avisa sobre término de sessões ativas. Kill/destroy encerra o filho PTY; não há promessa de limpar processos que deliberadamente se desacoplaram da sessão.

Teste Linux real verifica input, resize e saída; teste ANSI verifica cores/scrollback. Janela nativa capturada via X11. ConPTY Windows e backend macOS não executados neste ambiente. Nenhum software instalado no SO.
