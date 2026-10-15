# MODNAROK Patcher

Patcher desktop em Rust para atualizar o cliente Windows do MODNAROK.

## Implementado nesta base
- Interface desktop com botões de atualizar, troubleshooting e site oficial.
- Atualizações incrementais usando manifesto JSON remoto.
- Verificação SHA-256 e tamanho opcional.
- Atualização de caminhos relativos à pasta do cliente: arquivos em `data/`, `modnarok.grf`, `Modnarok.exe` e outros explicitamente listados no manifesto.
- Download temporário, verificação antes da instalação e backup do arquivo substituído.
- Bloqueio de caminhos inseguros (ex.: `../../arquivo.exe`).
- Consulta a JSON de troubleshooting hospedado online.
- Workflow GitHub Actions para compilar no Windows.

## Configurar hospedagem
Edite `src/config.rs` e troque `SEU-DOMINIO-AQUI` pelas URLs HTTPS reais. Não coloque o executável inteiro nem os GRFs neste repositório; hospede os arquivos de atualização no site/armazenamento de downloads.

Gere o manifesto a partir dos arquivos efetivamente publicados. Cada registro precisa conter caminho relativo no cliente, URL HTTPS, SHA-256 real e, de preferência, tamanho em bytes.

Exemplo de registro:
```json
{
  "path": "data/sprite/arquivo.spr",
  "url": "https://downloads.exemplo.com/patch/data/sprite/arquivo.spr",
  "sha256": "COLOQUE_O_SHA256_REAL_DE_64_CARACTERES",
  "size": 12345
}
```
Não use hashes fictícios em produção: o patcher recusará os arquivos que não corresponderem ao hash do manifesto.

## Troubleshooting online
Hospede `troubleshooting.json` em HTTPS. Formato:
```json
[
  {
    "id": "arquivo-ausente",
    "title": "Arquivo ausente ou corrompido",
    "symptoms": "O cliente acusa arquivo ausente.",
    "solution": "Execute a verificação de arquivos e consulte os links abaixo.",
    "keywords": ["arquivo", "ausente", "GRF"],
    "links": [{"label": "Suporte MODNAROK", "url": "https://modnarok.com.br/"}]
  }
]
```
Você poderá atualizar esse JSON no site sem recompilar o patcher.

## Compilação
Com Rust stable e ferramentas de compilação para Windows instaladas:
```powershell
cargo check
cargo test
cargo build --release
```
O executável será gerado em `target/release/modnarok-patcher.exe`.

## Limitações antes da distribuição pública
- URLs de produção ainda precisam ser configuradas.
- A tela atual informa quantos artigos online existem, mas ainda não renderiza os artigos completos dentro da janela; o botão abre a central web.
- O jogo precisa estar fechado para atualizar `Modnarok.exe`; substituição via helper e autoatualização do patcher ainda não estão implementadas.
- Esta versão usa hashes em manifesto HTTPS. Para proteção adicional contra adulteração do manifesto, recomendo assinatura digital do manifesto.
- A raiz do cliente é o diretório atual do processo. Até haver detecção/seleção explícita da pasta, execute o patcher da pasta do jogo.
- Teste numa cópia do cliente antes da distribuição.

## Update 00
O primeiro manifesto de produção deve ser gerado depois de decidir quais arquivos realmente mudaram. Evite enviar o cliente completo ou `data.grf` por engano; inclua somente os arquivos gerenciados pelo patcher.
