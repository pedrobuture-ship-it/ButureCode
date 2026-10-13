# Marco 20 — workspace e ambiente de desenvolvimento

Data: 13 de outubro de 2026. Autoria: Pedro Buture.

Prompts 06–20 implementados sobre a janela do marco 05. Top bar, navegação contextual, sidebar redimensionável, filesystem Explorer com watcher por polling, settings em camadas, host detection, onboarding, providers Rust/C/C++/Python/Java/Node/Go, Toolchain Manager e instalação assistida. [Roadmap](roadmap.md) e [limitações de toolchains](toolchains.md).

## Evidência de validação

Linux x86_64, Rust stable 1.99.0. Fmt check, Clippy workspace/all-targets com warnings como erros, testes do workspace e build aprovados. Dezessete testes exercitam renderização e modos de efeitos, geometria do shell, camadas/persistência, rejeição de secrets, detecção do host, subprocessos ausentes, timeout, isolamento de diretório de sondagem, desativação de auto-install em shims, planos allowlisted e validação negativa após gerenciador bem-sucedido. Explorer cobre lazy loading, ausência de sobrescrita, bloqueio de links simbólicos, watcher e preview reais.

QA nativa com eframe/Glow: nove capturas, incluindo Full/Reduced/Off, compacto, painéis ocultos, Explorer no próprio projeto, Toolchains com versões observadas, revisão do comando de instalação com confirmação desmarcada e onboarding. A geometria foi imposta pelo compositor: casos de 1920×1080 e 941×1030 na rodada final; não equivale a validação de todos os tamanhos solicitados. Não há instalação real durante a QA.

Inspeção verificou editor opaco, sidebar densa, navegação selecionada, árvore visível, resultados reais de ferramentas, revisão legível de comando e botão de execução desabilitado sem consentimento. Onboarding usa GlassSurface Modal. Capturas representativas em `design/previews/`. O exemplo `capture_shell` reproduz os fluxos sem gravar preferências pessoais.

## Limites

Windows/macOS não compilados ou executados. Suporte de gerenciadores e executáveis depende da plataforma; scripts Windows não são avaliados via shell. Instalação real, autenticação administrativa, cancelamento/rollback de pacotes e conflitos de gerenciadores não foram validados. Falhas são informadas; nunca assumidas como sucesso.

Explorer usa polling em dois segundos e preview UTF-8 limitado a 1 MiB; sem edição, undo ou PTY. Cópias podem ser parciais após falha de IO. A segurança de caminhos não isola um filesystem hostil com corridas concorrentes. Command Center permanece visual; profile e áreas posteriores são placeholders explícitos. Branch é lida de `.git/HEAD`, sem operações Git ou suporte completo a worktrees. Settings aceita apenas o schema implementado, com categorias futuras reservadas. Não foram medidos números de performance nem consumo de idle.

O ciclo termina aqui. Não implementar prompt 21 sem novas instruções.
