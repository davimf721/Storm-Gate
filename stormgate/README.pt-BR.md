# Storm Gate

**Runtime open source de compatibilidade para jogos Windows no macOS,
construído com Wine e tecnologias do ecossistema Proton.**

O Storm Gate busca ser o que "Proton para Mac" deveria significar: um runtime
gratuito e reproduzível que executa jogos Windows em Apple Silicon sem
assinatura, sem dependências proprietárias e sem exigir que o usuário conheça
prefixos do Wine, DLL overrides ou variáveis de ambiente.

```bash
stormgate doctor
stormgate steam install
stormgate steam start
stormgate game run 1091500
stormgate run ~/Games/MeuJogo/game.exe
```

> **Estado: pré-alfa (0.0.x, milestone BOOT).** O núcleo de orquestração, a
> CLI, os scripts de build e os testes existem; o runtime ainda não foi
> validado em hardware Apple Silicon real. Nenhum jogo está marcado como
> suportado. Veja o [roadmap](docs/roadmap.md).

## Como funciona

- **D3D10/11 → DXMT → Metal** (padrão)
- **D3D9 → DXVK → Vulkan → MoltenVK → Metal**
- **D3D12 → VKD3D-Proton → Vulkan → MoltenVK → Metal** (experimental)
- **CPU:** Wine x86_64 sob Rosetta 2

A CLI `stormgate` (Rust) inspeciona o executável, escolhe o backend gráfico,
cria um prefixo isolado por jogo, implanta as DLLs corretas, controla todo o
ambiente do Wine e grava logs de cada execução.

## Começando (desenvolvimento)

Requisitos: Mac Apple Silicon (M1 ou superior), macOS 14+, Rosetta 2, Xcode
Command Line Tools e Homebrew. Um MacBook Air M1 com 8 GB é suficiente;
mantenha ~40 GB livres ou use `STORMGATE_BUILD_DIR` em um SSD externo.

```bash
git clone https://github.com/davimf721/Storm-Gate
cd Storm-Gate/stormgate
make bootstrap      # verifica as ferramentas e mostra o que falta
make runtime        # compila Wine, DXMT, DXVK e MoltenVK e empacota
make install-runtime
make test
```

Antes de ter um runtime próprio, dá para testar com qualquer Wine existente:

```bash
STORMGATE_WINE=/opt/homebrew/bin/wine cargo run -p stormgate-cli -- run notepad.exe
```

## Princípios

- **Gratuito e aberto:** nada essencial depende de CrossOver, assinatura,
  nuvem ou backend proprietário. D3DMetal só como opcional instalado pelo
  usuário.
- **Reproduzível:** componentes fixados por commit, com `SHA256SUMS`,
  `sources.lock` e SBOM.
- **Fork pequeno:** reutilizar upstream e enviar correções de volta.
- **Compatibilidade legítima:** o Storm Gate nunca burla DRM ou anti-cheat.

A documentação técnica está em inglês para facilitar a colaboração
internacional: [docs/](docs/). Contribuições em português são bem-vindas em
issues e discussões.

Licença: Apache-2.0 OU MIT para o código próprio; componentes de terceiros
mantêm suas licenças ([LICENSES.md](LICENSES.md)).
