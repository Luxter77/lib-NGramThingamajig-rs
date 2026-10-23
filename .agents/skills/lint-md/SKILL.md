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

Lo excluido es el array `ignores` de ese archivo. Con `gitignore: true`, también queda afuera lo que cubre `.gitignore`. No duplicar ni reinterpretar esa lista.

`--fix` se puede anular desde esa configuración. Este jsonc no define `fix`. No agregarlo: si queda en false, el comando no escribe aunque lleve `--fix`.

Una regla que se queja suele ser intencional. No cambiar reglas ni ignores para silenciar hallazgos.
Si algo queda bloqueado y no hay salida razonable, pedirle al usuario que lo revise. No asumir que hay que cambiar la regla.

## Flujo

Correr desde la raíz, para que cargue `.markdownlint-cli2.jsonc`.

Siempre con `--fix`. No hay razón para correr sin `--fix`.

```powershell
bun x markdownlint-cli2 --fix "**/*.md"
```

El glob va entre comillas dobles. `--help` lo pide para que el shell no lo expanda antes que el linter. En Windows, comillas dobles.

Ese glob es el árbol de Markdown. No cambiarlo por `.`: `--help` mapea `.` a `*.{md,markdown}` del directorio actual, sin bajar al árbol. `**` pelado incluiría cualquier tipo de archivo.

No pasar un archivo suelto. Lo que importa es el arbol completo.

No pasar negaciones en el comando. El `#node_modules` del ejemplo de `--help` es para cuando no hay `ignores` en la configuración. Acá esa lista ya está en el jsonc.

No usar `--format`. Lee la entrada estándar, aplica arreglos y escribe el resultado en la salida estándar. No recorre el arbol.

`--fix` escribe en los archivos lo que puede autocorregir. La salida es la misma lista que un lint sin `--fix`: archivo, línea y regla. En esa lista marca cuáles hallazgos sí pudo arreglar.

Lo marcado ya quedó escrito. Lo que aparece sin esa marca es lo que hay que corregir a mano. Trabajar sobre esa salida. No hace falta otra corrida para ver qué quedó: sin `--fix` se repetiría la lista y se perdería la marca.

No lintar antes. No preguntar antes de este paso.

Si quedan hallazgos sin marca de arreglo, corregirlos a mano a partir de esa salida. Volver a correr el mismo comando con `--fix` hasta que quede limpio, o hasta un bloqueo sin arreglo razonable. En ese caso, preguntar. No cambiar reglas ni ignores para silenciar el hallazgo.

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

markdownlint no cubre todo esto. Después del lint, buscar en `*.md` desde la raíz del arbol. Cubrir también archivos sin trackear. Reportar hits y preguntar antes de editar.

Cómo buscar: ver qué hay en esta sesión y usar eso. Preferir herramientas del agente (Grep). Si no hay, PowerShell (`Select-String`) o `grep`. No asumir `rg` en el PATH. `git grep` solo mira archivos ya en el índice, así que no alcanza para archivos nuevos.

Búsquedas (glob `*.md`):

| Qué | Patrón |
| --- | --- |
| Negrita | `\*\*` |
| Em dash | `—` |
| En dash | `–` |
| Comillas curvas | `[“”‘’]` |

Para itálica con asteriscos, Grep `\*` y abrir los que no sean listas (`-`, `*`), fences, ni negrita.

Cómo arreglar, si es autorizado:

| Hallazgo | Qué hacer |
| --- | --- |
| `**texto**` | Sacar la negrita. Jerarquía con heading, lista, callout o wikilink. |
| `*texto*` | Pasar a `_texto_`. |
| `—` o `–` | Guion de teclado `-` solo si es un guion real. Si unía ideas, coma, punto y coma, o una oración aparte. |
| `“ ”` `‘ ’` | Comillas rectas `"` y `'`. |

También a ojo, sin grep obligatorio: headings en orden; un solo `#` de título cuando corresponde; wikilinks internos; URLs externas con `[texto](url)`.

## Cierre

Reportar: qué marcó `--fix` como arreglado, qué dejó esa misma salida sin arreglo, qué se editó a mano a partir de ella, y qué quedó abierto.

No commitear salvo que se lo pida.
