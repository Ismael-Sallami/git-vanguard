# GitVanguard

[![Licencia MIT](https://img.shields.io/badge/licencia-MIT-blue.svg)](LICENSE)
[![Plataforma](https://img.shields.io/badge/plataforma-Linux-orange.svg)](https://kernel.org)
[![Lenguaje](https://img.shields.io/badge/lenguaje-Rust%201.95+-red.svg)](https://www.rust-lang.org)
[![TUI Engine](https://img.shields.io/badge/motor-Ratatui%200.29-cyan.svg)](https://ratatui.rs)

**GitVanguard** (`vanguard` o alias `gv`) es un controlador de Git en terminal (TUI) de alto rendimiento, desarrollado desde cero en **Rust** exclusivamente para entornos **Linux**. 

Diseñado desde cero para ofrecer una velocidad de renderizado implacable (<5 ms por frame, sin sobrecargas de recolección de basura) y una suite completa de capacidades avanzadas: **Time Machine** para navegación visual y restauración instantánea del `reflog`, el escudo de seguridad proactivo **Sentinel Shield** contra fugas accidentales de secretos, un asistente integrado de **Conventional Commits** y un centro de mando nativo para **Git Worktrees**.

```
╭────────────────────────────────────────────────────────────────────────────────────────────────────────╮
│ GITVANGUARD v0.1   │ 1. Archivos │ 2. Ramas │ 3. Historial │ 4. Worktrees │ 5. Stash │ 6. Time Machine │
├──────────────────────────────────────────┬─────────────────────────────────────────────────────────────┤
│ Cambios de Trabajo [Unstaged: 2]         │ Inspección de Diferencias                                   │
│ ▶ [M] src/main.rs                        │    1 │ diff --git a/src/main.rs b/src/main.rs                  │
│   [?] config/secrets.env                 │    2 │ --- a/src/main.rs                                       │
│                                          │    3 │ +++ b/src/main.rs                                       │
├──────────────────────────────────────────┤    4 │ @@ -24,4 +24,8 @@                                       │
│ Cambios Preparados [Staged: 1]           │    5 │ +// Sentinel Shield: prevención activa de fugas         │
│   [M] src/sentinel.rs                    │    6 │ +pub fn scan_diff(diff: &str) -> Vec<SecretFinding>     │
╰──────────────────────────────────────────┴─────────────────────────────────────────────────────────────╯
  Tab: Cambiar Panel │ Space: Stage/Checkout │ c: Commit │ C: Conv. Commit │ U: Undo Reflog │ q: Salir
```

---

## Innovaciones Fundamentales

### 1. Sentinel Shield: Escudo Anti-Fugas Pre-Commit
A diferencia de otros clientes TUI que permiten commitear código ciegamente, GitVanguard incorpora un analizador estático en tiempo real sobre los cambios preparados (`staged`). Detecta e intercepta automáticamente:
- Claves de API (OpenAI, AWS Access Keys, Google Cloud, Stripe).
- Tokens personales de GitHub (`ghp_...`, `github_pat_...`) y Slack (`xoxb-...`).
- Claves privadas criptográficas (RSA, OpenSSH, PGP, certificados `.pem`).
- Ficheros de variables de entorno no ignorados (`.env`, `.env.local`, `credentials.json`).

Si se detecta un patrón de riesgo, GitVanguard bloquea preventivamente el commit y muestra un panel de auditoría indicando archivo, línea exacta y extracto ofuscado.

### 2. Time Machine: Navegador Visual de Reflog con Undo en 1 Tecla
El `reflog` de Git es el salvavidas definitivo ante errores, pero su inspección mediante CLI es engorrosa. Time Machine expone cada salto temporal (`checkout`, `commit`, `rebase`, `reset`, `merge`) con:
- Fecha relativa legible.
- Resumen de la acción ejecutada.
- Diff contextual instantáneo en el viewport contiguo.
- **Restauración instantánea con tecla `U`**: rebobina el repositorio al instante histórico exacto con confirmación protegida.

### 3. Hub de Mando para Git Worktrees
Los árboles de trabajo vinculados (*worktrees*) son el estándar de la ingeniería moderna para trabajar en ramas paralelas sin perder el estado del directorio de trabajo. GitVanguard ofrece un panel dedicado para:
- Visualizar todos los worktrees activos y sus ramas asociadas.
- Crear nuevos worktrees vinculados en un solo atajo (`n`).
- Desvincular y limpiar worktrees obsoletos (`d`).

### 4. Asistente Semántico Conventional Commits
Un asistente modal paso a paso para estandarizar el historial de versiones según la especificación *Conventional Commits*:
- Selector rápido de tipo: `feat`, `fix`, `refactor`, `perf`, `docs`, `style`, `test`, `chore`, `ci`.
- Ámbito opcional: `(auth)`, `(ui)`, `(engine)`.
- Indicador visual de *Breaking Change* (`!`).
- Contador de caracteres con aviso dinámico de la regla de oro 50/72 caracteres.

### 5. Zombie Branch Pruner
Identifica de forma automática e instantánea las ramas locales cuyos commits ya han sido completamente fusionados en la rama principal (`main`/`master`), permitiendo su depuración en lote de forma segura mediante confirmación.

---

## Instalación en Linux

GitVanguard se distribuye mediante un script instalador universal que detecta la arquitectura del procesador (`x86_64` o `aarch64`), configura los permisos y genera los alias necesarios:

```bash
curl -fsSL https://raw.githubusercontent.com/Ismael-Sallami/git-vanguard/main/install.sh | bash
```

### Compilación desde Código Fuente (Rust)
Si dispones de la cadena de herramientas de Rust:

```bash
# Clonar el repositorio
git clone https://github.com/Ismael-Sallami/git-vanguard.git
cd git-vanguard

# Compilar en modo release optimizado
cargo build --release

# Instalar en tu PATH local
cp target/release/vanguard ~/.local/bin/
ln -sf ~/.local/bin/vanguard ~/.local/bin/gv
```

---

## Guía de Atajos de Teclado

### Navegación Global
| Atajo | Acción |
| :--- | :--- |
| `Tab` / `BackTab` | Alternar secuencialmente entre paneles principales |
| `1` .. `6` | Salto directo a pestañas (1: Archivos, 2: Ramas, 3: Commits, 4: Worktrees, 5: Stash, 6: Time Machine) |
| `j` / `k` (o `↓` / `↑`) | Mover cursor verticalmente en listas |
| `J` / `K` | Desplazar el visor de diferencias (*Diff scroll*) |
| `?` | Abrir / cerrar manual interactivo de atajos |
| `q` | Salir de GitVanguard restaurando la terminal |

### Gestión de Archivos y Staging (Pestaña 1)
| Atajo | Acción |
| :--- | :--- |
| `h` / `l` (o `←` / `→`) | Alternar foco entre lista *Unstaged* y *Staged* |
| `Space` | Preparar o despreparar archivo seleccionado (*Stage / Unstage*) |
| `a` | Preparar todos los cambios (`git add -A`) |
| `u` | Despreparar todos los cambios (`git reset`) |
| `d` | Descartar cambios del archivo con confirmación de seguridad |
| `c` | Abrir diálogo de commit estándar |
| `C` | Abrir Asistente Semántico *Conventional Commits* |

### Ramas y Remotos (Pestaña 2)
| Atajo | Acción |
| :--- | :--- |
| `Space` / `Enter` | Cambiar a la rama seleccionada (*Checkout*) |
| `n` | Crear nueva rama a partir del commit actual |
| `d` | Eliminar rama local |
| `m` | Fusionar (*Merge*) la rama seleccionada en la rama activa |
| `r` | Aplicar *Rebase* sobre la rama seleccionada |
| `z` | *Zombie Branch Pruner*: purgar ramas locales ya fusionadas |
| `p` / `P` | Sincronizar: `git pull` / `git push` |
| `f` | Obtener actualizaciones remotas (`git fetch --all --prune`) |

### Worktrees, Stashes y Time Machine (Pestañas 4, 5 y 6)
| Atajo | Acción |
| :--- | :--- |
| `n` (en Worktrees) | Vincular un nuevo Git Worktree |
| `d` (en Worktrees) | Eliminar worktree secundario |
| `s` (en Stash) | Guardar cambios actuales en el stash (`git stash save -u`) |
| `p` / `a` (en Stash) | Aplicar y extraer (`pop`) o solo aplicar (`apply`) |
| `d` (en Stash) | Descartar entrada del stash (`drop`) |
| `U` (en Time Machine)| **Restaurar estado temporal**: rebobina HEAD al registro del reflog |

---

## Arquitectura Técnica

```
                       ┌───────────────────────────────┐
                       │     GitVanguard (vanguard)    │
                       └──────────────┬────────────────┘
                                      │
           ┌──────────────────────────┼──────────────────────────┐
           ▼                          ▼                          ▼
 ┌───────────────────┐      ┌───────────────────┐      ┌───────────────────┐
 │   Capa TUI Rust   │      │   Sentinel Shield │      │   Motor Git CLI   │
 │ ───────────────── │      │ ───────────────── │      │ ───────────────── │
 │ • Ratatui 0.29    │      │ • Regex Analyzer  │      │ • Subprocesos POSIX│
 │ • Crossterm 0.28  │      │ • Detección Claves│      │ • Porcelain v2    │
 │ • Doble búfer     │      │ • Ficheros .env   │      │ • Respeto GPG/SSH │
 │ • Render < 5 ms   │      │ • Bloqueo Commit  │      │ • Zero-overhead   │
 └───────────────────┘      └───────────────────┘      └───────────────────┘
```

1. **Sin Dependencias Dinámicas Pesadas:** Al ser un binario estático compilado en Rust nativo para Linux, no requiere intérpretes de Python, Node.js ni entornos de ejecución de Go.
2. **Compatibilidad Absoluta con el Ecosistema Git:** Al comunicarse de forma directa con el motor Git mediante streams optimizados (`status --porcelain=v2`, `for-each-ref`, `reflog`), preserva automáticamente el 100% de tus configuraciones globales (`~/.gitconfig`), claves SSH autenticadas, firmas criptográficas GPG/SSH y hooks del sistema.

---

## Licencia

Este proyecto está bajo la [Licencia MIT](LICENSE).

Desarrollado por **Ismael Sallami Moreno**  
*Doble Grado en Ingeniería Informática y ADE — Universidad de Granada*
