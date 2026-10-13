# BUTURE CODE — MASTER BUILD PROMPTS

## VISÃO GERAL

Você está construindo o **Buture Code**, uma IDE/workstation moderna escrita prioritariamente em Rust.

O projeto NÃO deve ser um clone do VS Code.

O Buture Code deve unir:

- editor de código nativo;
- performance;
- toolchains;
- compiladores;
- LSP;
- terminal;
- Git;
- debugger;
- Buture Doctor;
- Architecture View;
- Database Explorer;
- HTTP Client;
- Docker;
- desenvolvimento embedded;
- SSH;
- extensões;
- IA opcional.

O diferencial principal é:

> Buture Code entende não somente o código, mas também o ambiente de desenvolvimento.

IDENTIDADE VISUAL OFICIAL:

- glassmorphism;
- Apple-inspired;
- dark graphite;
- preto suave;
- superfícies translúcidas;
- blur discreto;
- verde esmeralda;
- design extremamente limpo;
- aparência premium;
- ferramenta de engenharia;
- nada gamer;
- nada cyberpunk;
- nada de glow exagerado;
- nada de dashboard SaaS genérico.

A referência visual oficial está em:

design/reference/buture-code-glass.png

Ela deve ser analisada antes de qualquer trabalho envolvendo UI.

Ela é uma referência de direção, NÃO deve ser copiada pixel a pixel.

O projeto deve possuir identidade própria.

---

# PROMPT 01 — BOOTSTRAP DO PROJETO

Crie do zero o projeto:

Buture Code

Utilize Rust stable.

Crie Cargo Workspace.

Estrutura inicial:

buture-code/
├── Cargo.toml
├── AGENTS.md
├── README.md
├── THIRD_PARTY_NOTICES.md
├── docs/
├── design/
│   └── reference/
└── crates/
    ├── buture-app
    ├── buture-core
    ├── buture-ui
    ├── buture-editor
    ├── buture-workspace
    ├── buture-project
    ├── buture-settings
    ├── buture-toolchains
    ├── buture-doctor
    ├── buture-search
    ├── buture-terminal
    ├── buture-git
    ├── buture-lsp
    ├── buture-debug
    ├── buture-tasks
    ├── buture-architecture
    ├── buture-database
    ├── buture-http
    ├── buture-docker
    ├── buture-hardware
    ├── buture-remote
    └── buture-ai

Antes de selecionar bibliotecas externas, analise:

- maturidade;
- manutenção;
- licença;
- compatibilidade;
- performance;
- suporte Windows/Linux/macOS.

Preferências:

- Rust;
- GPUI se adequado;
- Tokio;
- Serde;
- Tree-sitter;
- Rope para buffers;
- notify;
- tracing;
- thiserror;
- anyhow apenas em application boundaries.

Não copie código do VS Code ou Zed.

Não adicionar licença MIT ao projeto.

Não adicionar IA como autora, colaboradora ou coautora.

Crie:

docs/architecture.md

docs/roadmap.md

docs/design-system.md

docs/security.md

Faça o projeto compilar.

Execute:

cargo fmt
cargo clippy
cargo test
cargo build

Commit:

[feat] bootstrap Buture Code workspace


# PROMPT 02 — AGENTS.MD

Crie AGENTS.md para orientar qualquer agente que trabalhar no projeto.

Defina:

- Buture Code é software autoral;
- não adicionar licença MIT;
- não se adicionar como autor;
- não inserir Co-authored-by;
- commits pequenos;
- nunca commitar código que não compile;
- nunca quebrar feature existente sem necessidade;
- executar fmt/clippy/test;
- analisar imagem oficial antes de UI;
- evitar dependências desnecessárias;
- preservar performance;
- não executar operações destrutivas sem confirmação;
- não instalar software no SO silenciosamente.

Commits permitidos:

[feat]
[fix]
[refactor]
[design]
[perf]
[test]
[docs]
[chore]
[security]

Commit:

[docs] define repository agent rules


# PROMPT 03 — DESIGN SYSTEM BASE

Leia:

design/reference/buture-code-glass.png

Crie o design system oficial.

Conceito:

Buture Glass.

Buture Glass deve transmitir:

- precisão;
- engenharia;
- leveza;
- profundidade;
- elegância;
- foco.

Dark theme principal:

Buture Glass Dark.

Tokens:

background.base
background.deep
background.editor

surface.glass
surface.glassStrong
surface.panel
surface.elevated
surface.floating

border.subtle
border.normal
border.focus

text.primary
text.secondary
text.muted
text.disabled

accent.emerald
accent.emeraldHover
accent.emeraldMuted

success
warning
error
info

Adicionar:

spacing
radius
shadow
opacity
blur
animation durations

Não espalhar cores hardcoded.

Commit:

[design] establish Buture Glass design system


# PROMPT 04 — COMPONENTE GLASS SURFACE

Crie componente visual reutilizável:

GlassSurface

Variantes:

Subtle
Panel
Elevated
Floating
Modal

Permitir:

blur
opacity
border
radius
shadow

Centralizar implementação.

Não criar blur individual em cada componente.

Adicionar fallback:

VisualEffects::Full
VisualEffects::Reduced
VisualEffects::Off

No modo Off usar superfícies sólidas.

Commit:

[design] add reusable glass surface primitives


# PROMPT 05 — JANELA PRINCIPAL

Crie a janela principal do Buture Code.

Estrutura:

Top Bar

Activity Bar

Sidebar

Editor Area

Optional Right Inspector

Bottom Panel

Status Bar

Visual:

- vidro fumê;
- graphite;
- emerald;
- borda externa extremamente fina;
- radius elegante;
- shadows suaves;
- visual semelhante à referência.

O editor central NÃO deve ter blur atrás do texto.

Ele deve possuir superfície sólida de alta legibilidade.

Commit:

[feat] implement main IDE window shell


# PROMPT 06 — TOP BAR

Crie Top Bar premium.

Esquerda:

logo Buture Code
nome Buture Code

Centro:

Command Center

placeholder:

Search files, symbols, commands...

Direita:

Git branch
workspace controls
profile/settings placeholder

Visual:

glass;
baixo;
minimalista;
integrado à janela.

Adicionar shortcuts conforme SO:

macOS → Cmd

Windows/Linux → Ctrl

Commit:

[design] implement Buture Glass top bar


# PROMPT 07 — ACTIVITY BAR

Criar Activity Bar lateral.

Itens:

Explorer
Search
Source Control
Run & Debug
Database
Docker
Hardware

Settings na parte inferior.

Selected:

background esmeralda transparente;
icon esmeralda;
borda discreta.

Hover:

rápido e suave.

Não usar cores berrantes.

Commit:

[design] implement Buture activity navigation


# PROMPT 08 — SIDEBAR

Criar Sidebar redimensionável.

Header contextual.

Explorer como primeira implementação.

Árvore:

workspace
folders
files

Recursos:

expand/collapse
keyboard navigation
hover
selection
context menu

Não usar cards por arquivo.

Manter densidade de IDE profissional.

Commit:

[feat] implement workspace sidebar


# PROMPT 09 — FILE EXPLORER REAL

Implementar filesystem explorer funcional.

Recursos:

open folder
new file
new folder
rename
delete
copy
duplicate
copy path
refresh
collapse all
reveal

Lazy loading.

Filesystem watcher.

Não carregar árvores gigantes inteiramente sem necessidade.

Commit:

[feat] implement high performance file explorer


# PROMPT 10 — SISTEMA DE SETTINGS

Criar settings em camadas:

Default
User
Workspace

Categorias:

Appearance
Editor
Files
Terminal
Languages
Toolchains
Git
Debug
Database
Docker
Hardware
Remote
AI
Privacy

Persistência via formato legível.

Não guardar secrets diretamente.

Commit:

[feat] implement layered settings system


# PROMPT 11 — DETECTAR SISTEMA OPERACIONAL

Implementar buture-toolchains host detection.

Detectar:

Windows
Linux
macOS

Architecture:

x86_64
aarch64

Também detectar:

shell
PATH
home
common SDK directories

Nunca alterar o ambiente nesta etapa.

Commit:

[feat] detect host development environment


# PROMPT 12 — FIRST RUN ONBOARDING

Na primeira execução mostrar onboarding glass.

Etapa 1:

Welcome to Buture Code

Etapa 2:

System detected:

Linux
x86_64

ou correspondente.

Etapa 3:

What do you develop with?

Rust
C
C++
Python
Java
Node.js
TypeScript
Go
C#
Kotlin
PHP
Ruby

Usar selectable glass tiles.

Etapa 4:

Scan Environment

Etapa 5:

Environment Summary

Persistir conclusão.

Commit:

[feat] add first run developer onboarding


# PROMPT 13 — TOOLCHAIN ENGINE

Criar arquitetura:

ToolchainProvider

Cada provider deve detectar:

compiler
runtime
package manager
build tools
debugger
formatter
language server

Implementar:

Rust
C/C++
Python
Java
Node
Go

Commit:

[feat] implement extensible toolchain providers


# PROMPT 14 — RUST TOOLCHAIN

Detectar:

rustc
cargo
rustup
rust-analyzer
rustfmt
clippy
lldb/gdb quando aplicável.

Mostrar versão.

Suportar múltiplas versões.

Commit:

[feat] add Rust toolchain detection


# PROMPT 15 — C E C++

Detectar:

gcc
g++
clang
clang++
MSVC
cmake
make
ninja
gdb
lldb
clang-format

Windows/Linux/macOS.

Não assumir GCC como única opção.

Commit:

[feat] add C and C++ toolchain detection


# PROMPT 16 — PYTHON

Detectar:

python
python3
pip
pipx
uv
poetry
virtualenv

Detectar environments quando possível.

Nunca ativar ambiente automaticamente sem clareza.

Commit:

[feat] add Python environment detection


# PROMPT 17 — JAVA

Detectar:

JDK
javac
java
Maven
Gradle

Mostrar versão JDK.

Permitir futuras múltiplas installations.

Commit:

[feat] add Java toolchain support


# PROMPT 18 — NODE.JS

Detectar:

node
npm
pnpm
yarn
bun

Detectar version managers quando possível.

Não modificar Node global automaticamente.

Commit:

[feat] add JavaScript runtime detection


# PROMPT 19 — TOOLCHAIN MANAGER UI

Criar:

Settings > Development > Toolchains

Exemplo:

Rust

rustc 1.xx ✓
cargo 1.xx ✓
rust-analyzer ✓

C/C++

Clang ✓
CMake ✓
LLDB ✓

Python

Python 3.xx ✓
uv ✓

Node

Node 24 ✓
pnpm ✓

Ações:

Details
Install
Repair
Update

apenas quando suportado.

Commit:

[design] build toolchain manager interface


# PROMPT 20 — INSTALAÇÃO ASSISTIDA

Criar instalação de ferramentas faltantes.

REGRA:

Nunca instalar silenciosamente.

Fluxo:

Missing Tool

↓

Explain

↓

Show Installation Method

↓

Show Exact Command

↓

Ask Permission

↓

Execute

↓

Validate

Gerenciadores:

Windows:
winget se disponível.

Linux:
detectar distro.

Arch/Omarchy:
pacman
AUR apenas quando apropriado.

macOS:
brew quando disponível.

Não usar sudo silenciosamente.

Commit:

[feat] add assisted development environment setup


# PROMPT 21 — BUTURE DOCTOR CORE

Criar feature assinatura:

Buture Doctor.

Analisar:

toolchains
PATH
Git
project type
LSP
formatter
debugger
Docker
ports
environment
services

Status:

Ready
Needs Attention
Blocked

Cada diagnóstico:

title
description
severity
cause
solution

Commit:

[feat] implement Buture Doctor diagnostics engine


# PROMPT 22 — BUTURE DOCTOR UI

Criar interface premium glass.

Exemplo:

Buture Doctor

Environment: Needs Attention

Rust
✓ rustc
✓ cargo
⚠ rust-analyzer

Docker
✗ daemon unavailable

Project
⚠ DATABASE_URL missing

Permitir:

Fix Selected

Mas sempre mostrar confirmação.

Commit:

[design] create Buture Doctor workspace


# PROMPT 23 — PROJECT DETECTION

Detectar:

Cargo.toml
package.json
pyproject.toml
requirements.txt
go.mod
pom.xml
build.gradle
CMakeLists.txt
Makefile
.sln
.csproj

Suportar monorepo.

Criar:

ProjectKind

ProjectMetadata

BuildSystem

RunCandidate

Commit:

[feat] implement automatic project detection


# PROMPT 24 — DASHBOARD DO PROJETO

Criar Project Overview opcional.

Mostrar:

project
languages
toolchains
branch
build
tests
environment
services

Ações:

Run
Build
Test
Doctor
Terminal
Architecture

Usar glass discreto.

Nada de dashboard SaaS.

Commit:

[feat] add project overview workspace


# PROMPT 25 — TEXT BUFFER

Criar editor buffer robusto.

Utilizar Rope ou estrutura apropriada.

Implementar:

open
insert
delete
replace
line mapping
offset mapping
dirty
save
undo
redo

UTF-8.

LF/CRLF.

Adicionar benchmarks.

Commit:

[feat] implement scalable text buffer


# PROMPT 26 — EDITOR RENDERER

Renderizar código.

Implementar:

cursor
selection
line numbers
gutter
viewport virtualization
scrolling
horizontal scrolling
active line

Nunca renderizar documento completo sem necessidade.

Commit:

[feat] implement native code editor rendering


# PROMPT 27 — BUTURE EDITOR THEME

Criar syntax theme:

Buture Emerald Dark.

Características:

keywords:
violeta/soft purple

functions:
azul suave

types:
cyan discreto

strings:
verde suave

numbers:
amarelo discreto

comments:
cinza/verde muted

macros:
magenta discreto

Não transformar código em arco-íris.

Commit:

[design] add Buture Emerald syntax theme


# PROMPT 28 — TABS

Implementar editor tabs.

Recursos:

close
close others
close right
pin
preview
dirty indicator
restore closed

Visual integrado ao glass shell.

Commit:

[feat] add editor tabs


# PROMPT 29 — SPLIT EDITOR

Implementar:

split horizontal
split vertical
move tabs
multiple editor groups

Layout deve ser persistido.

Commit:

[feat] add split editor groups


# PROMPT 30 — TREE-SITTER

Integrar Tree-sitter.

Linguagens:

Rust
C
C++
Python
Java
JavaScript
TypeScript
Go
JSON
TOML
YAML
Markdown

Parsing incremental.

Não bloquear UI.

Commit:

[feat] add incremental syntax parsing


# PROMPT 31 — EDIÇÃO AVANÇADA

Adicionar:

multi cursor
multiple selections
duplicate line
move line
comment
uncomment
auto close
bracket matching
indent
outdent
word navigation
select occurrence
folding foundation

Commit:

[feat] add advanced editing operations


# PROMPT 32 — SEARCH

Implementar:

Ctrl/Cmd+F

workspace search

replace

regex

case sensitive

whole word

.gitignore support

globs

Streaming de resultados.

Commit:

[feat] add fast project search


# PROMPT 33 — COMMAND PALETTE

Criar Command Palette glass.

Atalho:

Cmd+Shift+P
Ctrl+Shift+P

Adicionar fuzzy search.

Commands:

Open File
Run Project
Run Doctor
Toggle Terminal
Git Commit
Change Theme
Settings

Architecture:

CommandRegistry

Commit:

[feat] add command palette


# PROMPT 34 — QUICK OPEN

Ctrl/Cmd+P

Pesquisar arquivos rapidamente.

Mostrar:

filename
relative path

Fuzzy search.

Visual igual ao Command Palette.

Commit:

[feat] add quick file navigation


# PROMPT 35 — KEYBINDINGS

Criar keybinding engine.

Defaults por SO.

Allow overrides.

Detect conflicts.

Preparar chords.

Commit:

[feat] add customizable keyboard shortcuts


# PROMPT 36 — TERMINAL

Implementar PTY real.

Recursos:

shell detection
multiple terminals
split
rename
kill
cwd
ANSI
resize
scrollback
copy/paste

UI glass.

Terminal em superfície sólida para legibilidade.

Commit:

[feat] add integrated terminal


# PROMPT 37 — TASK RUNNER

Detectar:

cargo
npm
pnpm
make
cmake
maven
gradle
go

Criar:

Task

command
args
env
cwd
dependencies

UI para selecionar tasks.

Commit:

[feat] implement task runner


# PROMPT 38 — SMART RUN

Criar botão universal:

RUN

Inferir:

Cargo → cargo run

Node → scripts

Python → provável entrypoint

Go → go run

Java → build system

C/C++ → build configured

Se ambíguo:

perguntar.

Guardar preferência.

Commit:

[feat] add intelligent run workflow


# PROMPT 39 — LSP CORE

Implementar Language Server Protocol client.

Suportar:

initialize
open
change
save
close
shutdown

Começar com rust-analyzer.

Arquitetura independente de linguagem.

Commit:

[feat] implement LSP client architecture


# PROMPT 40 — INTELLISENSE

Adicionar:

completion
hover
go to definition
references
rename
signature help
document symbols
workspace symbols
code actions
diagnostics

Diagnostics no:

gutter
Problems panel

Commit:

[feat] add language intelligence features


# PROMPT 41 — FORMATTERS

Suportar:

rustfmt
clang-format
ruff
black
prettier
gofmt

Format Document.

Format Selection.

Format on Save.

Commit:

[feat] add formatter registry


# PROMPT 42 — PROBLEMS PANEL

Criar Problems.

Errors
Warnings
Info

Cada item:

file
line
column
message
source

Click abre arquivo.

Commit:

[feat] add diagnostics panel


# PROMPT 43 — GIT CORE

Implementar:

repository detection
status
branch
stage
unstage
commit
checkout
create branch
fetch
pull
push

Nunca destruir mudanças sem confirmação.

Commit:

[feat] add native Git integration


# PROMPT 44 — SOURCE CONTROL UI

Criar:

Changes
Staged Changes

File list.

Commit box.

Branch indicator.

Use visual glass discreto.

Commit:

[design] create Buture source control workspace


# PROMPT 45 — DIFF

Criar:

inline
side-by-side

Stage hunk.

Unstage hunk.

Discard com confirmação.

Commit:

[feat] add Git diff editor


# PROMPT 46 — GIT GRAPH

Mostrar:

commits
branches
merges
tags

Visual minimalista.

Sem arco-íris.

Commit click → Inspector.

Commit:

[feat] add Git history graph


# PROMPT 47 — COMMIT ASSISTANT

Analisar staged diff.

Sugerir:

[feat]
[fix]
[refactor]
[perf]
[test]
[docs]
[chore]

Usuário confirma.

Nunca commit automático.

Commit:

[feat] add commit message assistant


# PROMPT 48 — DEBUGGER CORE

Criar Debug Adapter abstraction.

Estruturas:

DebugSession
Breakpoint
Thread
StackFrame
Scope
Variable

Avaliar DAP.

Não amarrar a uma linguagem.

Commit:

[feat] add debugger foundation


# PROMPT 49 — DEBUGGER UI

Criar:

Variables
Watch
Call Stack
Breakpoints
Debug Console

Toolbar flutuante glass:

Continue
Pause
Step Over
Step Into
Step Out
Restart
Stop

Commit:

[feat] add graphical debugger workspace


# PROMPT 50 — MEMORY INSPECTOR

Adicionar:

memory addresses
hex
ASCII
goto
search

Future:

stack
heap
pointer graph

Somente ativar se debugger suportar.

Commit:

[feat] add debugger memory inspector


# PROMPT 51 — HEX VIEWER

Criar visualizador binário.

OFFSET
HEX
ASCII

Recursos:

search bytes
goto offset
endianness
integer interpretation

Não carregar arquivos gigantes completamente.

Commit:

[feat] add native hex viewer


# PROMPT 52 — ARCHITECTURE VIEW

Criar segunda grande feature assinatura:

Buture Architecture.

Visual:

canvas escuro;
glass nodes;
linhas discretas;
emerald highlight.

Mostrar inicialmente:

Rust crates
modules
dependencies
imports

Node click:

abre arquivo.

Adicionar:

zoom
pan
search
fit view

Commit:

[feat] add project architecture graph


# PROMPT 53 — ARCHITECTURE MULTI-LANGUAGE

Adicionar providers:

JavaScript
TypeScript
Go
Python
Java

Diferenciar:

internal dependency
external dependency
module dependency

Nunca inventar relação.

Commit:

[feat] expand architecture providers


# PROMPT 54 — DEPENDENCIES

Criar Dependencies Explorer.

Rust:
Cargo.

Node:
package.json.

Mostrar:

direct
transitive
version

Preparar future vulnerability info.

Commit:

[feat] add dependency explorer


# PROMPT 55 — PORT MANAGER

Detectar processos escutando portas.

Mostrar:

3000 Node
5432 PostgreSQL
6379 Redis
8080 Java

Ações:

Open Browser
Copy
Kill

Kill exige confirmação.

Commit:

[feat] add port manager


# PROMPT 56 — ENVIRONMENT MANAGER

Detectar:

.env
.env.local
.env.development

Mostrar keys.

Valores secretos:

•••••••

Reveal manual.

Nunca colocar secrets em logs.

Commit:

[feat] add secure environment manager


# PROMPT 57 — DATABASE EXPLORER

Criar provider inicialmente para PostgreSQL.

Mostrar:

Connections
Schemas
Tables
Views

Credenciais via secure OS storage quando disponível.

Commit:

[feat] add database explorer


# PROMPT 58 — SQL EDITOR

Criar:

SQL Editor

Run Query

Results Grid

Execution Time

Cancel

History

Resultados grandes devem ser limitados/paginados.

Commit:

[feat] add SQL development workspace


# PROMPT 59 — HTTP CLIENT

Criar client HTTP integrado.

GET
POST
PUT
PATCH
DELETE

Params
Headers
Auth
Body

Response:

status
time
size
headers
body

Não copiar Postman.

Commit:

[feat] add integrated HTTP client


# PROMPT 60 — DOCKER

Detectar Docker.

Mostrar:

containers
images
ports
status

Actions:

start
stop
restart
logs
terminal

Detectar compose.

Commit:

[feat] add Docker development tools


# PROMPT 61 — HARDWARE DEVICES

Criar Buture Hardware.

Detectar serial devices.

Mostrar:

port
vendor
product

Arquitetura para:

ESP32
STM32
RP2040
Arduino
RISC-V

Commit:

[feat] add hardware device manager


# PROMPT 62 — SERIAL MONITOR

Criar:

Device
Baud
Connect
Logs
Input
Send

Baud presets.

Autoscroll.

Timestamp.

Commit:

[feat] add serial monitor


# PROMPT 63 — EMBEDDED DETECTION

Detectar:

ESP-IDF
PlatformIO
Arduino
embedded CMake
embedded Rust

Mostrar:

target
board
toolchain
serial
programmer

Somente dados confirmados.

Commit:

[feat] detect embedded projects


# PROMPT 64 — FIRMWARE FLASH

Criar FirmwareFlasher abstraction.

Workflow:

Build
Detect Target
Show Command
Confirm
Flash
Stream Logs

Jamais flash automático.

Commit:

[feat] add firmware flashing workflow


# PROMPT 65 — SSH REMOTE

Criar remote development via SSH.

Recursos:

hosts
connect
terminal
files
edit
save

Respeitar known_hosts.

Nunca aceitar host key silenciosamente.

Commit:

[feat] add SSH remote development


# PROMPT 66 — NEW PROJECT WIZARD

Criar wizard glass.

Languages:

Rust
C
C++
Python
Java
TypeScript
Go

Templates:

CLI
Library
Desktop
API
Embedded
Empty

Mostrar files que serão criados.

Commit:

[feat] add project creation wizard


# PROMPT 67 — THEMES

Criar:

Buture Glass Dark

Buture Glass Light

Dark é padrão.

Light NÃO deve ser simples inversão.

Adicionar accent:

Emerald default.

Optional:

Blue
Purple
Orange
Monochrome

Commit:

[design] add Buture theme system


# PROMPT 68 — VISUAL EFFECTS

Settings:

Appearance > Visual Effects

Full
Reduced
Off

Full:
blur
transparency
shadows

Reduced:
menos blur

Off:
solid surfaces

UI deve continuar bonita.

Commit:

[perf] add scalable visual effects modes


# PROMPT 69 — PERFORMANCE DO GLASS

Profile:

blur
transparent layers
shadows
resize
scroll
terminal
command palette

Não aplicar blur em:

editor text
file rows
terminal text
every tab

Garantir responsividade.

Commit:

[perf] optimize glass rendering


# PROMPT 70 — UI ADAPTATIVA

Testar:

1280x720
1440x900
1920x1080
2560x1440

Quando estreito:

hide activity labels
shrink sidebar
hide optional inspector
prioritize editor

Commit:

[design] improve adaptive desktop layout


# PROMPT 71 — HIGH DPI

Testar:

100%
125%
150%
200%

Garantir:

sharp text
sharp icons
correct hitboxes
correct cursor
proper border widths

Commit:

[design] improve HiDPI rendering


# PROMPT 72 — TEST EXPLORER

Criar providers:

cargo test
pytest
go test
npm test quando detectável

Mostrar:

passed
failed
ignored
duration

Run:

single
file
suite
all

Commit:

[feat] add test explorer


# PROMPT 73 — PROJECT DOCTOR 2.0

Integrar:

Toolchains
LSP
Formatters
Debugger
Docker
Ports
Environment
Tests
Build
Git
Database dependencies

Mostrar:

Ready
Needs Attention
Blocked

Commit:

[feat] expand Buture Doctor


# PROMPT 74 — EXTENSION SYSTEM

Projetar plugin API.

Evitar plugins nativos arbitrários.

Preferir:

WASM/WASI

Criar manifest.

Permissions:

filesystem
network
commands
workspace
UI

Sandbox por padrão.

Documentar security model.

Commit:

[feat] design secure extension architecture


# PROMPT 75 — AI PROVIDERS

Somente agora adicionar IA.

Criar abstraction:

AIProvider

Suportar futuramente:

OpenAI
Anthropic
Google
Ollama
LM Studio
OpenAI-compatible

Não hardcode keys.

Commit:

[feat] add AI provider architecture


# PROMPT 76 — BUTURE AI

Criar painel glass.

Actions:

Explain
Fix
Refactor
Generate Tests
Ask Project

Context:

Selection
Current File
Open Files
Chosen Project Files

Mostrar exatamente o contexto enviado.

Nunca modificar automaticamente.

Gerar patch.

Commit:

[feat] add contextual AI assistant


# PROMPT 77 — PATCH REVIEW

Criar componente universal:

Patch Review

Usado por:

AI
Doctor
Refactors
Automated fixes

Mostrar diff.

Actions:

Apply
Apply Hunk
Reject
Edit

Commit:

[feat] add unified patch review


# PROMPT 78 — WELCOME SCREEN FINAL

Criar tela inicial baseada na imagem oficial.

Mostrar:

Buture Code

Engineering without friction.

Quick Actions:

Open Folder
Clone Repository
New Project
Connect Remote

Recent Projects

Development Environment:

Rust ✓
C/C++ ✓
Python ✓
Node ✓
Docker ✓
Git ✓

Pouco conteúdo.

Muito espaço negativo.

Glass premium.

Commit:

[design] finalize Buture Code welcome experience


# PROMPT 79 — COMMAND CENTER FINAL

Unificar busca:

Files
Symbols
Commands
Settings
Projects

Top bar.

Fuzzy search rápido.

Commit:

[feat] add unified command center


# PROMPT 80 — NOTIFICATIONS

Criar notifications glass.

Success
Warning
Error
Info
Progress

Exemplo:

Build finished
Docker unavailable
rust-analyzer restarted

Cor só como detalhe.

Commit:

[design] add refined notifications


# PROMPT 81 — ERROR HANDLING

Falha em:

LSP
Terminal
Git
Docker
Database

não pode derrubar IDE.

Adicionar:

structured errors
retry
restart
logs

Commit:

[fix] improve subsystem fault isolation


# PROMPT 82 — PERFORMANCE PASS

Testar:

10.000 files
large files
large Git repositories
30 tabs
multiple terminals
large search
Architecture View

Aplicar:

lazy loading
virtualization
debounce
background work
cancellation

Commit:

[perf] optimize large project workloads


# PROMPT 83 — SECURITY REVIEW

Auditar:

shell commands
toolchain installation
SSH
Database credentials
.env
HTTP tokens
AI
plugins
Docker
Git

Evitar:

command injection
secret logging
unsafe path handling
silent destructive actions

Commit:

[security] harden Buture Code


# PROMPT 84 — CROSS PLATFORM

Auditar:

Windows
Linux
macOS

Especialmente:

paths
PTY
shell
processes
filesystem
toolchains
serial
SSH

Centralizar diferenças em platform layer.

Commit:

[refactor] harden cross platform support


# PROMPT 85 — TESTES

Criar suite robusta.

Testar:

buffer
editor operations
project detection
toolchains
doctor
settings
Git
LSP
tasks
architecture
workspace restore

Separar:

unit
integration

Commit:

[test] expand automated test coverage


# PROMPT 86 — UI POLISH

Compare toda interface com:

design/reference/buture-code-glass.png

Pergunte para cada tela:

"Isso parece parte do Buture Code?"

Corrigir:

spacing
radius
opacity
typography
shadows
alignment
icons
visual hierarchy

Remover:

cards desnecessários
glow excessivo
neon
placeholders
componentes genéricos

Commit:

[design] polish Buture Glass interface


# PROMPT 87 — UX PASS

Revisar:

keyboard navigation
focus
tooltips
hover
context menus
empty states
errors
loading
reduced motion
accessibility

Não deixar mouse como única forma de interação.

Commit:

[design] improve accessibility and UX


# PROMPT 88 — LOGO E BRANDING

Finalize branding.

Nome:

Buture Code

Criar suporte a logo asset localizado em:

assets/branding/

Não criar marcas semelhantes a Apple, VS Code ou Zed.

Integrar logo:

Welcome
Top Bar
About
Installer futuramente

Commit:

[design] finalize Buture Code branding


# PROMPT 89 — ABOUT

Criar About Buture Code.

Mostrar:

Buture Code
version
build
Rust version
platform

Links configuráveis futuramente.

Mostrar third party notices.

Não inserir créditos de IA como autoria.

Commit:

[feat] add About Buture Code


# PROMPT 90 — RELEASE ALPHA

Preparar alpha.

Executar:

cargo fmt --check

cargo clippy

cargo test

cargo build --release

Verificar:

TODOs
FIXMEs
panic
unwrap
dead code
mock
placeholders
secrets
unused dependencies

Criar:

docs/alpha-readiness.md

Classificar:

BLOCKER
HIGH
MEDIUM
LOW

Corrigir blockers.

Commit:

[chore] prepare Buture Code alpha


# PROMPT 91 — AUDITORIA VISUAL FINAL

Abra todas as áreas disponíveis.

Comparar:

Welcome
Editor
Explorer
Search
Git
Terminal
Doctor
Toolchains
Architecture
Database
HTTP
Docker
Hardware
Settings
Debug
AI

Garantir:

mesma identidade
mesma iconografia
mesmo spacing
mesmas surfaces
mesma linguagem glass

Nenhuma tela deve parecer produto diferente.

Commit:

[design] complete visual milestone


# PROMPT 92 — AUDITORIA DE ARQUITETURA

Revisar arquitetura inteira.

Procurar:

coupling excessivo
crates circulares
duplicação
globals
blocking UI calls
overengineering
dependencies desnecessárias

Refatorar sem alterar comportamento.

Documentar decisões importantes.

Commit:

[refactor] improve Buture Code architecture


# PROMPT 93 — PERFORMANCE FINAL

Executar profiling.

Medir:

startup
memory
open project
open large file
scroll
search
LSP
terminal
Git
Architecture View

Documentar:

docs/performance.md

Não inventar números.

Commit:

[perf] establish Buture Code performance baseline


# PROMPT 94 — DOCUMENTAÇÃO

Atualizar:

README.md

docs/architecture.md
docs/editor.md
docs/toolchains.md
docs/doctor.md
docs/lsp.md
docs/debugger.md
docs/git.md
docs/hardware.md
docs/architecture-view.md
docs/extensions.md
docs/ai.md
docs/security.md
docs/performance.md

Documentação deve refletir implementação real.

Commit:

[docs] document Buture Code alpha


# PROMPT 95 — PRODUTO FINAL COESO

Agora revise o produto como uma experiência completa.

Fluxo esperado:

Launch Buture Code

↓

First Run

↓

Detect OS

↓

Select Languages

↓

Scan Toolchains

↓

Configure Environment

↓

Open / Clone / Create Project

↓

Detect Stack

↓

Buture Doctor

↓

Editor

↓

Run / Build / Test

↓

Debug

↓

Git

↓

Architecture

Tudo deve parecer parte de uma única experiência.

O Buture Code deve ser percebido como:

> uma workstation de desenvolvimento nativa em Rust com ambiente, ferramentas e arquitetura do projeto integrados.

Os três pilares principais são:

1. Buture Doctor
2. Zero-config Toolchains
3. Architecture View

A identidade principal é:

Buture Glass.

Faça apenas ajustes necessários para tornar tudo coerente.

Commit:

[chore] complete Buture Code alpha milestone