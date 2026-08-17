# Timesheet Automático para Advogados — Design

Data: 2026-08-17
Status: aprovado para implementação

## Problema

Escritórios que cobram por hora dependem de cada advogado registrar manualmente
quanto tempo dedicou a cada caso. O preenchimento manual é tardio e feito "de
memória", produzindo dados imprecisos e consumindo tempo faturável.

O sistema monitora automaticamente a atividade do advogado no computador e gera
um rascunho de timesheet diário, que o advogado revisa e aprova.

## Decisões fundamentais

| Decisão | Escolha | Motivo |
|---|---|---|
| O que é capturado | Metadados de foco de janela | Captura de tela violaria sigilo profissional (OAB) e criaria repositório de imagens de risco desproporcional |
| Modelo de implantação | App local por advogado; dado bruto nunca sai da máquina | Só agregados aprovados são compartilhados |
| Plataforma alvo | Windows | Realidade dos escritórios; API mais simples e sem permissão especial |
| Framework de UI | Tauri 2 | Núcleo continua Rust puro; tabela editável é território de HTML/CSS |
| Persistência local | SQLite via `rusqlite` (`bundled-sqlcipher`) | Sem dependência externa no instalador; tracker é síncrono e não precisa de runtime async |
| Filtro de captura | Allowlist de apps | Denylist falha para o lado inseguro em app não previsto |
| Servidor central | **Fora do MVP** | Nada dos passos 1–6 depende dele; a tese difícil é o tracking |

### Ambiente de desenvolvimento

O desenvolvimento acontece em macOS e o alvo é Windows. O design isola as APIs
de sistema operacional atrás de um trait justamente por isso: sem essa
separação, nenhuma lógica de negócio poderia ser executada ou testada na
máquina de desenvolvimento.

Uma VM Windows é necessária a partir do passo 3 da ordem de implementação.

## Arquitetura

Workspace Cargo com cinco crates:

| Crate | Responsabilidade | Depende de SO |
|---|---|---|
| `ts-core` | Modelos de domínio, sessionizer, matcher, agregador | Não |
| `ts-platform` | Trait `ActivityMonitor` + implementações Windows/macOS | Sim |
| `ts-storage` | SQLite/SQLCipher, migrations, repositórios, retenção | Não |
| `ts-export` | Importação de casos (CSV) e exportação do timesheet | Não |
| `ts-app` | Tauri: bandeja, comandos, frontend de revisão | — |

`ts-core` não conhece SQL nem sistema operacional: recebe amostras, devolve
intervalos e agregados. Toda a lógica que pode contar hora errada mora ali, e
é integralmente testável com `cargo test` em qualquer plataforma.

### Fluxo de dados

1. **Thread de tracking** (dedicada, síncrona, período de 1s) consulta o
   `ActivityMonitor`: qual janela está em foco e há quanto tempo não há input.
2. **Sessionizer** (`ts-core`) converte amostras em intervalos. Fecha o
   intervalo quando o app ou título muda, quando o ocioso cruza o limiar, ou
   quando detecta suspensão da máquina.
3. **Persistência**: intervalo fechado é gravado no SQLite. Apenas append.
4. **Agregação**: o matcher classifica cada intervalo em um caso, agrupa por
   caso e produz um rascunho de timesheet do dia.
5. **Revisão**: o advogado corrige e aprova na UI.
6. **Exportação**: os registros aprovados saem em CSV/planilha.

### Precisão temporal

Três regras que o sessionizer precisa respeitar, e que são a fonte mais comum
de erro neste tipo de sistema:

- **Ocioso fecha retroativamente.** `GetLastInputInfo` informa há quanto tempo
  não há input. Ao cruzar o limiar (5 min), o intervalo é fechado no instante
  do *último input*, não no instante da detecção. Fechar em "agora" adicionaria
  o limiar inteiro a cada pausa.
- **Duração vem do relógio monotônico**; os timestamps de parede servem apenas
  para exibição. Ajuste de NTP ou mudança de fuso não pode produzir duração
  negativa ou inflada.
- **Suspensão** aparece como salto do relógio entre duas amostras. Salto muito
  maior que o período de polling fecha o intervalo na última amostra válida.

## Privacidade e coleta

### Allowlist de aplicativos

Somente apps explicitamente listados têm o título de janela gravado. Todos os
demais gravam apenas nome do app e duração. Um app novo e não previsto grava o
mínimo por padrão — o erro cai sempre para o lado seguro.

No MVP a allowlist é um arquivo de configuração local, distribuído com o app e
editável.

### Regra específica para navegadores

Allowlist sozinha não resolve o navegador: sem captura de URL, o título da aba
do PJe e o título da aba do e-mail pessoal chegam pelo mesmo canal, e o segundo
contém dados pessoais do próprio advogado.

Para apps marcados como navegador, o título passa pelo matcher **em memória** e
só é gravado em disco se casar com um caso conhecido ou com um sistema
conhecido. Sem match, persiste como `Navegador — não classificado` com a
duração.

Custo aceito: blocos não classificados chegam à revisão sem contexto do que
eram. Contrapartida: navegação pessoal nunca é escrita em disco.

### Captura de URL

Fora de escopo. A leitura da barra de endereço exigiria extensão de navegador
ou Accessibility API; o título da aba já vem embutido no título da janela; e
uma extensão que lê URLs é o componente mais difícil de defender numa
auditoria.

### Controle do usuário

- **Pausa visível e sempre disponível** na bandeja, com o ícone refletindo o
  estado atual sem exigir clique. É o que distingue ferramenta de produtividade
  de software de vigilância, tanto na percepção de quem é monitorado quanto na
  análise trabalhista.
- **Tela de primeira execução** listando o que é coletado, o que permanece na
  máquina e o que é exportado. Permanece acessível depois da instalação.
- **Meus dados**: o advogado visualiza tudo que está gravado sobre ele e pode
  apagar qualquer intervalo antes da aprovação (soft delete). Atende os
  direitos do titular na LGPD e reaproveita a tela de revisão.

### Proteção em repouso

- Banco em `%LOCALAPPDATA%`, **nunca** em `%APPDATA%` (Roaming). Em ambiente
  com Active Directory, perfis roaming sincronizam para compartilhamento de
  rede — o banco bruto sairia da máquina silenciosamente, quebrando a garantia
  central do projeto.
- SQLCipher com chave protegida por DPAPI, amarrada à conta Windows do
  usuário.
- Limite explícito: protege contra roubo do equipamento e contra outro usuário
  da mesma máquina. **Não** protege contra malware executando como o próprio
  advogado.

### Retenção

Intervalos brutos são apagados após 30 dias por rotina automática na abertura
do app. Agregados aprovados permanecem.

## Schema local

```sql
activity_interval(
  id, started_at, ended_at,
  duration_secs,        -- do relógio monotônico, NÃO derivado dos timestamps
  app_name,
  window_title,         -- NULL fora da allowlist ou navegador não classificado
  end_reason,           -- focus_change | idle | suspend | shutdown | paused
  case_id, match_source, -- cnj | alias | manual | none
  deleted_at            -- soft delete via "meus dados"
)

known_case(id, cnj_number, client_name, label, active)
case_alias(case_id, alias)          -- strings que aparecem em títulos de janela
app_allowlist(app_name, capture_title, is_browser)

timesheet_entry(
  id,                   -- UUID; chave de idempotência para sync futuro
  work_date, case_id, minutes, description,
  approved_at, revision, exported_at
)
```

`duration_secs` gravado separadamente dos timestamps não é redundância: os
timestamps exibem, a duração fatura, e só a segunda é confiável.

O UUID em `timesheet_entry` é mantido mesmo sem servidor: custo zero agora,
e é o que torna o envio idempotente quando o sync existir.

Migrations versionadas, aplicadas na inicialização.

## Matching

Duas camadas:

1. **Regex do padrão CNJ** `NNNNNNN-DD.AAAA.J.TR.OOOO`, com **validação do
   dígito verificador** (módulo 97, ISO 7064, conforme Resolução CNJ 65/2008).
   Sem a validação, qualquer sequência numérica em título de planilha vira
   falso positivo.
2. **Busca por alias** de caso ou cliente no título.

Aliases são dados, não código: o escritório corrige matching errado editando o
CSV, sem atualizar o app.

## Testes

- **Sessionizer** com `Clock` injetado e sequências sintéticas de amostras.
  Casos obrigatórios: troca de foco, ocioso retroativo, suspensão, pausa
  manual, virada de meia-noite, app fora da allowlist. Cobertura integral sem
  tocar em sistema operacional.
- **Matcher** com tabela de títulos reais mapeados para o caso esperado,
  incluindo negativos obrigatórios: título de e-mail pessoal não pode casar
  com nada.
- **Storage** contra SQLite em memória, com migrations aplicadas a cada teste.
- **`ts-platform`** tem implementação fake para os testes; a implementação
  Windows é verificada manualmente em VM.

## Ordem de implementação

1. `ts-core`: sessionizer + testes (roda em macOS, sem dependência de SO)
2. `ts-platform`: trait + implementação macOS — primeiro dado real
3. `ts-platform`: implementação Windows — **antes** de storage
4. `ts-storage`: schema, migrations, SQLCipher/DPAPI, retenção
5. Matcher e agregação
6. `ts-app`: bandeja, pausa, tela de revisão, primeira execução, exportação

O passo 3 vem cedo de propósito: é a única peça não testável na máquina de
desenvolvimento, e descobrir no passo 6 que o trait não acomoda a API do
Windows custaria caro.

## Fora de escopo no MVP

- Servidor central e sincronização
- Captura de URL de navegador
- Relatórios, gráficos e dashboard de gestão
- Linux e macOS como plataformas de produção (macOS existe apenas como
  ambiente de desenvolvimento)
- Integração com sistema jurídico do escritório

## Questões em aberto

- **Servidor central versus integração.** Se o escritório já usa sistema
  jurídico (Astrea, Projuris, ADVBOX, SAJ, Legal One, CPJ), casos e
  faturamento já vivem lá, e o servidor deve nascer como camada de integração
  em vez de produto próprio. Decisão adiada até o tracking estar provado.
- **Política de transparência e consentimento** alinhada à LGPD e ao Código de
  Ética da OAB precisa ser redigida antes de qualquer implantação real em
  escritório. Não bloqueia o desenvolvimento; bloqueia a implantação.
- **Limiar de ocioso** fixado em 5 minutos por ora. Pode precisar de ajuste
  após uso real — leitura longa de documento sem input é atividade legítima.
