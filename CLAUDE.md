# Easy Timesheet

Timesheet automático para advogados que faturam por hora. Rastreia metadados de
janela em foco (nunca captura de tela) e gera um rascunho diário que o advogado
revisa e aprova.

## Como trabalhar neste projeto

**O usuário escreve todo o código.** Claude atua como guia: explica APIs, aponta
armadilhas, revisa, desenha arquitetura, escreve testes quando pedido. Não
escrever código de produção sem pedido explícito naquele momento — autorização
não é permanente.

É a primeira aplicação desktop do usuário; a experiência prévia em Rust é
web/APIs com Axum. O objetivo é aprender construindo.

Ao explicar, prefira descrever assinatura, crate e comportamento esperado a
colar implementação. Trechos curtos ilustrando uma API são aceitáveis.

## Restrições invioláveis

**Sigilo (OAB) e LGPD.** Dado bruto de atividade nunca sai da máquina do
advogado. Título de janela é o dado mais sensível do sistema. Antes de propor
qualquer coisa que grave, transmita ou registre em log, verifique se vaza
título ou URL.

**Dev em macOS, produção em Windows.** Nenhuma lógica de negócio acoplada a API
de SO. O usuário tem um PC Windows físico para os passos que exigem.

## Arquitetura

Workspace Cargo. Crates atuais em `crates/`:

| crate | responsabilidade | depende de SO |
|---|---|---|
| `ts-core` | `model.rs` (tipos), `sessionizer.rs` (máquina de estados) | não |
| `ts-platform` | trait `ActivityMonitor`, `FakeMonitor`, `macos.rs` | sim |

Planejadas: `ts-storage` (SQLite/SQLCipher), `ts-export` (CSV), `ts-app` (Tauri).

Pipeline: amostras → sessionizer → matcher → redator → storage → agregação →
revisão → exportação. **O matching acontece no fechamento do intervalo**, não na
agregação, porque a regra do navegador precisa saber se casou antes de decidir
se o título pode ser gravado.

Design completo (local, fora do git — `/docs` está no `.gitignore`):
`docs/superpowers/specs/2026-08-17-timesheet-automatico-design.md`

## Estado atual

**Passo 1 — `ts-core`: sessionizer** — 8 testes verdes. Cobre troca de foco,
fechamento retroativo por ocioso, prioridade do ocioso sobre troca de foco, não
reabrir durante ausência, retomada com idle realista, início com máquina já
ociosa, amostra impossível sem pânico.

**Passo 2 — `ts-platform`** — 2 testes verdes. Trait + `FakeMonitor` prontos
(fatia 1). `MacOsMonitor` com `idle_ms()`, `mono_ms()` e `frontmost_app()`
(fatias 2 e 3). Falta a fatia 4: título da janela via Accessibility.

## Roteiro

1. ✅ `ts-core`: sessionizer
2. 🔄 `ts-platform`: trait + macOS (falta fatia 4)
3. ⬜ `ts-platform`: implementação Windows — **antes** de storage, para validar
   o formato do trait cedo
4. ⬜ `ts-core`: matcher (regex CNJ com dígito verificador + aliases) e agregação
5. ⬜ `ts-storage`: schema, migrations, SQLCipher/DPAPI, retenção de 30 dias
6. ⬜ `ts-app`: Tauri — bandeja, pausa, tela de revisão, primeira execução

## Pendências conhecidas

- Renomear crates para `tsf-*` (decidido, não executado)
- Três testes do sessionizer faltam, cada um com uma mudança estrutural:
  suspensão (`wall_ms` na `FocusSample`), virada de meia-noite (`local_day`),
  pausa manual (variante `Paused`)
- `close_now()` no sessionizer: o bloco aberto se perde quando o app encerra
- `FocusSample` provavelmente ganha `bundle_id` quando a allowlist existir

## Decisões fechadas

- **Allowlist de apps, não denylist.** Denylist falha para o lado inseguro em
  app não previsto.
- **Regra do navegador**: título de navegador só é persistido se casar com um
  caso conhecido. Sem match, vira `Navegador — não classificado`. Custo aceito:
  bloco não classificado chega à revisão sem contexto.
- **Sem captura de URL.** Extensão de navegador é o componente mais difícil de
  defender numa auditoria, e o título da aba já vem no título da janela.
- **Banco em `%LOCALAPPDATA%`, nunca `%APPDATA%`** (Roaming sincroniza para
  rede em domínio AD — o banco bruto sairia da máquina).
- **Servidor central fora do MVP.** Casos entram por CSV, timesheet sai em
  planilha. Se o escritório já usa sistema jurídico (Astrea, Projuris, SAJ…), o
  servidor nasce como integração, não como produto.
- **Limiar de ocioso: 5 minutos.** Ocioso abaixo do limiar conta integralmente;
  acima, não conta nada — nem o começo. Fechar em `último_input + limiar` faria
  toda pausa faturar 5 minutos fantasma.

## Armadilhas descobertas

- `mono_ms - idle_ms` precisa de `saturating_sub` e travamento em
  `start_mono_ms`. Fim de intervalo nunca antes do começo.
- `idle_ms` **nunca é exatamente 0** em produção (polling de 1s). Comparar com
  `< LIMIAR`, jamais `== 0`.
- `GetLastInputInfo` (Windows) devolve `DWORD` de 32 bits que estoura a cada
  49,7 dias. Subtração com `wrapping_sub` — ali a volta é a matemática correta,
  ao contrário do caso acima.
- `directories`: usar `data_local_dir()`, **não** `config_dir()` (que devolve
  Roaming).
- Permissão de Acessibilidade no macOS é concedida ao **terminal/IDE** de onde
  o binário roda, não ao binário. Checar `AXIsProcessTrusted()` e falhar alto.
- Core Foundation: função com `Copy`/`Create` no nome devolve objeto que é seu —
  precisa de `CFRelease`. Vazamento num laço de 1 amostra/segundo cresce rápido.
- `CGEventSourceStateID` = `1` (HID), não `0` — o `0` inclui eventos sintéticos,
  e um jiggler manteria o advogado "ativo" de cadeira vazia.
- Nada de formatar identificadores (`capitalizedString` corromperia o bundle id
  usado no matching da allowlist).
- Test double compartilhado entre crates **não** pode estar atrás de
  `#[cfg(test)]` — some para as outras. Usar feature `test-utils`.
- Nunca logar título de janela. `tracing` é o caminho mais fácil de vazar tudo
  que o resto do design protege.

## Comandos

```bash
cargo test                                    # workspace inteiro
cargo test -p ts-core                         # uma crate
cargo clippy --all-targets                    # manter limpo
cargo run -p ts-platform --example idle       # bancada manual do macOS
```

Código de fronteira com o SO se verifica **observando**, não assertando —
`examples/` existe para isso. Tudo acima da fronteira se testa com `FakeMonitor`.
