# Buture Glass

Referência oficial: `design/reference/buture-code-glass.png`, examinada antes da UI. Direção: grafite, preto suave, vidro fumê, esmeralda contida, bordas finas, sombras suaves e tipografia legível. Identidade autoral, sem copiar componentes de outras IDEs.

O editor central terá fundo sólido; blur nunca ficará atrás de código. Efeitos serão centralizados, com Full, Reduced e Off. Cores, espaçamento, raios, sombras, opacidade, blur e duração de animações serão tokens, sem constantes visuais espalhadas nos componentes.

## Tokens implementados — Buture Glass Dark

Fonte única: `crates/buture-ui/src/theme.rs`. Os nomes públicos Rust usam snake_case, correspondendo aos nomes do briefing:

| Grupo | Tokens |
| --- | --- |
| background | base, deep, editor |
| surface | glass, glassStrong, panel, elevated, floating |
| border | subtle, normal, focus, width |
| text | primary, secondary, muted, disabled |
| accent | emerald, emeraldHover, emeraldMuted |
| semantic | success, warning, error, info |
| spacing | xs 4, sm 8, md 12, lg 16, xl 24, xxl 32 |
| radius | small 6, panel 10, floating 14, window 16 |
| shadow | color, offset, blur, spread |
| opacity | subtle, panel, elevated, floating, modal, reduced |
| blur | full, reduced, backdropSize |
| motion | fast 100 ms, normal 180 ms, slow 280 ms |
| typography | caption, body, title, display, code |

Cores armazenadas como RGBA, independentes do toolkit. Editor `#12171c`, texto primário `#e8eff0` e esmeralda `#64dcb0`. Espaçamento em pontos lógicos, durações em `Duration`. Os estados sem efeitos sempre preservam o fundo sólido do editor.
