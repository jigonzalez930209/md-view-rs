# MD View — loop film

Variant C — seamless 1:1 loop (1080 × 1080, ~13.2 s, GitHub Light). `tools/gen.py` chooses the loop points by pixel match; footage is copied from `../linkedin/assets/cap/light`.

Generate with `python3 tools/gen.py`, then `npx hyperframes@0.9.58 check` and
`npx hyperframes@0.9.58 render -o ../out/loop-1x1.mp4`.
