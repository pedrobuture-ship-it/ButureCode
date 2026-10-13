# Buture Code

Workstation de desenvolvimento nativa em Rust, criada por **Pedro Buture**.

O Buture Code une código e ambiente de desenvolvimento. Buture Doctor, toolchains e Architecture View são os pilares planejados; ainda não estão implementados.

## Desenvolvimento

Rust stable, sem instalação automática de ferramentas no sistema:

```sh
cargo run -p buture-app
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

## Estado

Bootstrap de um Cargo workspace com 22 crates. Integrações futuras estão reservadas como módulos vazios, sem funcionalidades simuladas. Os prompts 01–05 definem o primeiro marco visual.

- [Arquitetura](docs/architecture.md)
- [Roadmap](docs/roadmap.md)
- [Design system](docs/design-system.md)
- [Segurança](docs/security.md)
- [Dependências e licenças](THIRD_PARTY_NOTICES.md)

## Direitos

Copyright © 2026 Pedro Buture. Todos os direitos reservados. Nenhuma licença de código aberto foi concedida ao código autoral deste projeto. As dependências mantêm suas próprias licenças.
