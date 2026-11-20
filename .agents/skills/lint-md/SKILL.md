---
name: lint-md
description: >-
  Revisa Markdown con markdownlint-cli2 y busca
  negrita, itálica con asteriscos, em/en dashes y comillas tipográficas.
  Usar al hacer lint, al cerrar un bloque de ediciones `.md`, o cuando
  se pide revisar, validar o limpiar.
---
# Lint

Usar `bun x`, no `npx`. Si hace falta runtime JS, leer también `bun-instead-of-node`.
Para sintaxis Obsidian, leer `obsidian-markdown`.

## Configuración

La corrida carga la configuración que lista `--help`, desde la raíz. Acá es `.markdownlint-cli2.jsonc`.

`--fix` se puede anular desde esa configuración. Este jsonc no define `fix`. No agregarlo: si queda en false, el comando no escribe aunque lleve `--fix`.

Una regla que se queja suele ser intencional. No cambiar reglas ni ignores para silenciar hallazgos.
Si algo queda bloqueado y no hay salida razonable, pedirle al usuario que lo revise. No asumir que hay que cambiar la regla.

## Flujo

Correr desde la raíz para que cargue `.markdownlint-cli2.jsonc`.

Usar el script de lint completo que ejecuta markdownlint y verifica problemas tipográficos:

```bash
bun .agents/skills/lint-md/scripts/lint.ts
```

Este script:

1. Ejecuta `markdownlint --fix` en todo el árbol de Markdown
2. Escanea todos los archivos markdown en busca de problemas estructurales (siempre):
   - Paréntesis de wikilink desbalanceados (WIK001) - validado, no hard error
   - Paréntesis de enlaces/imágenes Markdown desbalanceados (WIK002)
   - Enlaces con ruta absoluta sin protocolo (WIK003)
   - Wikilinks con formato de ruta (WIK004) - validado, no hard error
   - Validación de YAML frontmatter (YML001)
   - Patrones de escape requeridos (ESC001, ESC002)
3. Escanea problemas tipográficos (saltea archivos en `TYPOGRAPHIC_EXCLUDE`):
   - Em dashes (`—`)
   - En dashes (`–`)
   - Comillas dobles curvas/inteligentes (`“ ”`)
   - Comillas simples curvas/inteligentes (`‘ ’`)
   - Negrita (`**texto**`)
   - Cursiva con asteriscos (`*texto*`)
4. Reporta todos los problemas con rutas de archivo y números de línea

El script sale con código 0 si limpio, 1 si hay problemas.

El script es la interfaz principal, no ejecutar `markdownlint` a mano. El script respeta los `ignores` del `.markdownlint-cli2.jsonc` (incluyendo `.agents/skills/lint-md/**/*.md`).

WIK001 y WIK004 se reportan como advertencia (`⚠`), no como error (`✗`). No impiden que el lint pase, pero señalan wikilinks que pueden revisarse.

## Portar desde otro proyecto

Al adaptar `lint-md` desde otro repositorio, separar lo general de Markdown de lo específico del dominio. El usuario decide caso a caso; no asumir.

| Mantener (general de Markdown) | Descartar (dominio específico) |
| --- | --- |
| Tipografía: em/en dashes, comillas curvas, negrita `**`, cursiva con `*` | Escapes de tags de Obsidian (`#AO####`, `#RRGGBB`) |
| Validación de frontmatter YAML (YML001) | Registro de tags (`tag-registry.json`, TAG001) |
| Enlaces con ruta absoluta sin protocolo (WIK003) | Wikilinks a artículos raw (`raw/articles/...`, WIK005) |
| Balance de paréntesis wikilink y markdown (WIK001, WIK002, WIK004) como advertencia | Referencias a skills o archivos de otro proyecto en `TYPOGRAPHIC_EXCLUDE` |
| Patrones de escape requeridos (ESC001, ESC002) como general de Markdown | Cualquier regla que solo tenga sentido si el repo usa wikis/Obsidian |

Si el repo no usa wikilinks `[[...]]`, mantener WIK001/WIK004 como advertencia (validan si existen, no exigen que solo se usen wikilinks). No desactivar reglas para silenciar hallazgos sin preguntar al usuario.

Dos callouts de Obsidian seguidos: una línea en blanco dispara MD028; un `>` en el medio los fusiona y en lectura solo queda el de arriba. El `--fix` no lo resuelve bien. Separar con un comentario HTML vacío entre ellos:

```markdown
> [!note] Primero
> Texto.

<!-- -->

> [!note] Segundo
> Texto.
```

Vale para otros bloques `>` consecutivos que tengan que verse aparte y MD028 se queje. No desactivar la regla por esto.

Opcional, whitespace de Git:

```powershell
git diff --check
```

## Tipografía y énfasis

El script de lint revisa esto automáticamente en todos los archivos markdown (salvo los de `TYPOGRAPHIC_EXCLUDE`). Patrones que detecta:

- Em dashes (`—`)
- En dashes (`–`)
- Comillas dobles curvas/inteligentes (`“ ”`)
- Comillas simples curvas/inteligentes (`‘ ’`)
- Negrita (`**texto**`)
- Cursiva con asteriscos (`*texto*`)

Los archivos en `TYPOGRAPHIC_EXCLUDE` (`AGENTS.md`, `CLAUDE.md`, y este mismo skill) se excluyen de las comprobaciones tipográficas automáticas únicamente porque contienen ejemplos negativos para ilustrar las reglas. Las comprobaciones estructurales (WIK001-004, YML001, ESC001-002) corren en todos los archivos.

La única excepción permitida para estos patrones prohibidos es mostrar ejemplos negativos explícitos al documentar la regla misma (por ejemplo, ilustrar qué patrón buscar o mostrar "si está así, pasarlo a esta otra forma"). Fuera de la cita literal del patrón prohibido, el texto propio de estos archivos excluidos debe cumplir todas las reglas tipográficas: no usar negrita como énfasis en su propia redacción o listas, no usar cursiva con asteriscos, no usar guiones largos para puntuación propia ni comillas curvas.

Cómo arreglar, si es autorizado:

| Hallazgo | Qué hacer |
| --- | --- |
| `**texto**` | Quitar la negrita. Jerarquía con heading, lista, callout o wikilink. |
| `*texto*` | Pasar a `_texto_`. |
| `—` o `–` | Guion de teclado `-` solo si es un guion real. Si unía ideas, coma, punto y coma, o una oración aparte. |
| `“ ”` `‘ ’` | Comillas rectas `"` y `'`. |

También a ojo, sin grep obligatorio: headings en orden; un solo `#` de título cuando corresponde; wikilinks internos; URLs externas con `[texto](url)`. Code blocks que son templates o ejemplos también deben cumplir, los agentes los reproducen.

Estas comprobaciones tipográficas corresponden a las reglas de `AGENTS.md`: sin guiones em/en, sin comillas curvas, sin negrita como énfasis, y cursiva con guiones bajos (`_texto_`) en lugar de asteriscos (`*texto*`).

## Cierre

Reportar: qué marcó `--fix` como arreglado, qué dejó esa misma salida sin arreglo, qué se editó a mano a partir de ella, y qué quedó abierto.

No commitear salvo que se lo pida.

## Lint de alcance de proyecto (project-scoped)

El usuario puede pedir que el lint solo cubra archivos bajo `.agents/` (p.ej., "blanket exclude for the stuff in the repo, except those under .agents"). Para eso:

1. En `runMarkdownlint()`: cambiar el patrón de `"**/*.md"` a `".agents/**/*.md"`.
2. En `findMarkdownFiles()`: cambiar el patrón de `"**/*.md"` a `".agents/**/*.md"`.
3. El `.markdownlint-cli2.jsonc` ya excluye `.agents/skills/lint-md/**/*.md` vía `ignores`; no tocar.

Esto hace que:
- `markdownlint` solo revise archivos bajo `.agents/` (respeta `ignores` del config).
- El escaneo tipográfico/estructural propio del script recorra el mismo conjunto.
- El script funcione desde cualquier directorio de trabajo (ver siguiente sección).

## Detección de raíz del proyecto (ROOT)

El script debe calcular `ROOT` (raíz del repo) a partir de su propia ubicación, no de `process.cwd()`. El patrón anterior `resolve(__dirname, "../../../..")` falla si el script se mueve o se ejecuta desde fuera del repo.

### Implementación robusta (ya en `scripts/lint.ts`):

```ts
function findProjectRoot(startDir: string): string {
  let current = startDir;
  while (current !== "/") {
    try {
      if (
        readFileSync(resolve(current, "AGENTS.md"), "utf-8").length > 0 ||
        readFileSync(resolve(current, ".git"), "utf-8").length > 0
      ) {
        return current;
      }
    } catch {
      // Archivo no existe, seguir subiendo
    }
    current = resolve(current, "..");
  }
  return startDir;
}

const ROOT = findProjectRoot(__dirname);
```

Sube desde `__dirname` hasta encontrar `AGENTS.md` o `.git`. Usa `try/catch` para ignorar `ENOENT`. Esto permite correr `bun /ruta/absoluta/scripts/lint.ts` desde cualquier cwd.
