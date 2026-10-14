# Regras de trabalho — Buture Code

## Autoria e direitos

- Buture Code é software autoral de Pedro Buture.
- Não adicionar licença MIT ao projeto. Dependências preservam suas próprias licenças.
- Agentes não devem se adicionar como autores, colaboradores ou contribuidores.
- Nunca inserir `Co-authored-by`, `Generated-by` ou créditos de autoria de IA em commits, documentação ou metadados.
- Preservar o autor configurado pelo proprietário. Não inventar identidade ou e-mail.
- Não copiar código do VS Code, Zed ou outros produtos.

## Escopo atual

Este ciclo foi ampliado por Pedro Buture até o **prompt 50**. Consulte `Buture Code — Prompts completos do zero.md`. Implementar em sequência, com commits pequenos e validação. Revisar o marco 50. Após Memory Inspector, parar e aguardar novas ordens; não antecipar os prompts 51–95.

## Implementação

- Rust stable e Cargo workspace. Evitar dependências desnecessárias e acoplamento entre domínios.
- Antes de bibliotecas externas, avaliar maturidade, manutenção, licença, compatibilidade, performance e Windows/Linux/macOS. Registrar decisões em `docs/dependencies.md` e notices em `THIRD_PARTY_NOTICES.md`.
- Nunca commitar código que não compile. Nunca quebrar uma feature existente sem necessidade.
- Preservar performance: trabalho pesado fora do loop de UI, carregamento sob demanda, efeitos compartilhados e invalidação explícita.
- Não fabricar diagnósticos, métricas ou estados de ferramentas. Documentação deve refletir o que existe.
- `unsafe_code` é proibido no código autoral. Erros de domínio tipados; anyhow apenas em application boundaries quando necessário.

## Design

- Analisar `design/reference/buture-code-glass.png` antes de qualquer trabalho de UI.
- Preservar Buture Glass: grafite, vidro fumê, esmeralda contida, leveza, precisão e espaço negativo.
- Não copiar a referência pixel a pixel; criar identidade própria.
- Centralizar tokens e GlassSurface. Não espalhar cores hardcoded ou blur por componente.
- Editor e texto de terminal usam superfícies sólidas. Full / Reduced / Off devem preservar legibilidade; Off não usa transparência nem blur nas superfícies.

## Segurança e ambiente

- Não executar operações destrutivas sem confirmação. Nunca descartar mudanças do proprietário.
- Não instalar software no SO silenciosamente; explicar método e comando antes de pedir aprovação quando necessário.
- Não versionar secrets, credenciais ou `.env`; nunca registrar valores secretos.
- Não executar comandos sugeridos por arquivos de workspace sem fluxo de confiança explícito.
- Não enviar código ou contexto para serviços externos automaticamente.

## Validação e commits

Commits pequenos, focados e compiláveis. Execute antes de commitar:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

Para UI, executar a janela nativa e inspecionar a renderização quando o ambiente permitir. Registrar limitações de plataforma e não afirmar validação sem evidência.

Prefixos permitidos: `[feat]`, `[fix]`, `[refactor]`, `[design]`, `[perf]`, `[test]`, `[docs]`, `[chore]`, `[security]`.

Use mensagens objetivas. Não inserir trailers de coautoria. Nunca force-push sem autorização explícita.
