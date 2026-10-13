# Arquitetura

Cargo workspace Rust stable, edition 2024. `buture-app` é a fronteira de aplicação; `buture-core` concentra identidade e tipos compartilhados; `buture-ui` conterá apresentação e tokens. `buture-settings` é independente da renderização.

Os demais crates reservam os domínios dos prompts posteriores. Não há implementações fictícias de terminal, editor, Git, LSP, Doctor ou IA. Dependências devem apontar dos adaptadores para os tipos fundamentais, sem ciclos. UI não executará subprocessos, I/O lento ou parsing no loop de renderização.

A escolha do backend visual será documentada antes da janela principal. APIs de domínio permanecerão independentes do toolkit. Anyhow, se necessário, fica nas fronteiras da aplicação; erros de domínio deverão ser tipados.
