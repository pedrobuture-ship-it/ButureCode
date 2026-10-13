# Projetos — prompts 23–24

Detecção em worker de Cargo, package.json, Python, Maven/Gradle, Go e CMake/Makefile. JSON/TOML são realmente analisados; formatos adicionais usam leitura limitada de campos/manifests. Monorepos podem produzir múltiplos projetos. Travessia respeita ignore, não segue symlinks e limita profundidade a 16, entradas a 100 mil e projetos a 1024.

Project Overview mostra nome, tipo, linguagens, build system e manifests detectados. A branch é leitura de `.git/HEAD`. Scripts candidatos são metadados, nunca executados automaticamente. Builds/tests aparecem como não executados. Run, Build, Test, Terminal e Architecture permanecem indisponíveis neste marco. Doctor, Toolchains e abertura de manifest são ações reais.

Detecção é heurística e limitada; não resolve todos os formatos/DSLs ou subprojetos ignorados. Manifests inválidos ou leitura falha geram evidência de erro, sem inventar um projeto pronto.
