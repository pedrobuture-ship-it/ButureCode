# Buture Glass

Referência oficial: `design/reference/buture-code-glass.png`, examinada antes da UI. Direção: grafite, preto suave, vidro fumê, esmeralda contida, bordas finas, sombras suaves e tipografia legível. Identidade autoral, sem copiar componentes de outras IDEs.

O editor central tem fundo sólido; blur nunca ficará atrás de código. Efeitos são centralizados, com Full, Reduced e Off. Cores, espaçamento, raios, sombras, opacidade, blur e duração de animações são tokens, sem constantes visuais espalhadas nos componentes.

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

## GlassSurface

`GlassSurface` concentra composição e política visual. Variantes Subtle, Panel, Elevated, Floating e Modal; builders para blur, opacity, border, radius e shadow. `paint` serve a regiões de layout e `show` envolve componentes com padding consistente.

Full usa um backdrop **interno à aplicação**, gerado com os tokens, e blur gaussiano separável em resolução 320×200. Cada raio é calculado uma vez e reutilizado como textura; nenhuma leitura de framebuffer por widget. Reduced limita blur a 4 e remove sombras; Off força alpha 255, blur 0 e ausência de sombra, inclusive sobre overrides. UVs usam o viewport global para evitar descontinuidades entre painéis.

Não há captura do wallpaper ou blur de janelas do SO. Transparência nativa do desktop depende do compositor e não é requisito do shell; o vidro deste marco é composto dentro da janela. Texto e editor permanecem fora do pipeline de blur.
