# Catedral Sainte-Cécile — requisitos, decisiones y especificación

Documento de referencia para reconstruir el diorama como una catedral gótica grande,
**solo interior**. Orden de prioridad cuando algo se contradice:

1. Requisitos del proyecto (sección 1).
2. Decisiones del autor (sección 2).
3. Adaptación a solo-interior (sección 3).
4. Especificación original de Manus (sección 5).

---

## 1. Requisitos del proyecto

Texto original del enunciado:

> No pueden usar librerías externas a su lenguaje de programación. Su nota máxima es 100 puntos.
>
> Puntos:
>
> - [30 puntos] Criterio subjetivo. Por qué tan compleja sea su escena
> - [20 puntos] Criterio subjetivo. Por qué tan visualmente atractiva sea su escena
> - [20 puntos] Por implementar rotación en su diorama y dejar que la camara se acerque y aleje
> - [5 puntos] por cada material diferente que implementen, para un máximo de 5 (piensen en los diferentes tipos de bloques en minecraft)
>   - Para que el material cuente, debe tener su propia textura, y sus propios parametros para albedo, specular, transparencia y reflectividad
> - [10 puntos] por implementar refracción en al menos uno de sus materiales (debe tener sentido contextual en su escena)
> - [5 puntos] por implementar reflexión en al menos uno de sus materiales
> - [20 puntos] por implementar un skybox para su material

### Cómo los cubre la catedral

| Requisito | Cómo se cumple |
|---|---|
| Complejidad (30) | Nave de 6 tramos con naves laterales, crucero, coro, ábside octogonal, bóvedas de crucería, pilares, arquerías, triforio ciego, claristorio, 12 props de detalle + los 3 esqueletos, ~40 luces. |
| Atractivo visual (20) | Noche a la luz de velas + luna fría; el vitral del ábside proyecta su color sobre los novios; mármol pálido y bóveda oscura (ver imágenes de referencia). |
| Rotación y zoom (20) | Cámara orbital (flechas / arrastrar, W/S / rueda) sobre vistas predefinidas (C). Extra opcional: exploración en primera persona (F), no cuenta para la nota. |
| 5 materiales (25) | Cada uno con textura propia y sus propios albedo/specular/transparencia/reflectividad: **Piedra/mármol** (M02), **Madera** (M10), **Vidrio** (M05–M08, refractivo), **Metal** (M09, reflectivo, textura PNG), **Hueso con musgo** (M13, los esqueletos). |
| Refracción con sentido (10) | El vitral del ábside (IOR 1.5). |
| Reflexión (5) | Hierro de candelabros, cruz y emplomado. |
| Skybox (20) | Cielo nocturno procedural con luna y estrellas (`skybox.rs`), visible por el claristorio abierto y el rosetón. |

**Punto abierto:** el proyecto usa las crates `minifb` (ventana), `nalgebra-glm`
(vectores) e `image` (cargar el PNG del metal). El código del curso usa estas mismas
crates, así que se asume que están permitidas; confirmar con el catedrático si hay duda.

---

## 2. Decisiones del autor

- **Solo interior, definitivo.** No se modela ni se ve el exterior.
- Los novios y el oficiante eran esqueletos con musgo. **En la revisión de la fase 7 el autor los cambió por personas vivas:** novia, novio y oficiante. El material Hueso con musgo de la rúbrica sigue en la escena, en el panel del relicario.
- **Piso abierto**, sin bancas, solo unos pocos reclinatorios.
- **Luna baja en el este, a unos 20° sobre el horizonte**: entra por el vitral del ábside y su luz de colores cae sobre los novios en el crucero.
- **Candelabros de pie a lo largo de la nave**, como en la imagen de referencia.
- **Claristorio abierto** al cielo nocturno, sin vidrio.
- Primera persona a escala humana normal (ojo a 1.6 m).
- Las texturas de bloques no llevan contorno oscuro (`edge_darken = 0`).
- Mantener el código simple: sin duplicación y sin optimizaciones que no se hayan medido.

---

## 3. Adaptación a solo-interior (manda sobre la sección 5)

### Se elimina (solo exterior)

- E01 (plaza).
- Parte alta de las torres y agujas (E08 arriba de la bóveda del vestíbulo, E09).
- Troneras del campanario y almenas (E28, E29).
- Contrafuertes (E16).
- Todos los techos (E17, E18, E18a–d): el cascarón de la bóveda cierra el interior.
- Vistas de cámara 4 y 5.

**Se conserva la cara interior de todo:**

- el muro oeste con el rosetón abierto y las puertas cerradas;
- las aberturas del claristorio;
- el vitral del ábside.

El grid termina en las caras exteriores de los muros.

### Correcciones al spec

1. **Arquería de la nave (E13).** Como está escrita arranca en el piso y llega a 15.2 m, más alto que la cumbrera de las naves laterales (14.8 m). Queda así:
   - arranque a **5.6 m** (28 vox) y clave a **11.0 m** (55 vox), igual que la clave de la bóveda de la nave lateral;
   - entre 12.0 y 15.6 m, **triforio ciego**: nichos oscuros (M03) de 0.4 m de profundidad, uno por medio tramo;
   - el muro del claristorio (E11) queda macizo desde la clave de la arquería hasta el alféizar del claristorio, salvo esos nichos.
2. **Luna.** Una sola luz direccional, baja en el este (20°), más el ambiente frío existente. Reemplaza las 4 lunas del spec.
3. **Vestíbulos en las bases de las torres.** Las puertas laterales (E27) entran a las bases de las torres, que pasan a ser vestíbulos:
   - bóveda con clave a unos 10 m;
   - un arco hacia cada nave lateral en `y = 8 m`;
   - el cañón de 29 m no se modela.
4. **Techos no definidos en el spec:**
   - nártex: bóveda de cañón apuntado con arranque a 12 m y clave a **18 m**. No puede ser más baja: el rosetón (centro a 14 m, Ø 4.8 m) tiene que quedar adentro. Se abre a la nave con un arco de 6.4 × 16.8 m, por el que el rosetón se ve desde la nave;
   - vestíbulos de las torres: arranque a 7.2 m y clave a 10 m;
   - brazos del crucero: cañón con arranque a 11.2 m y clave a **15.2 m** (muros a 16.8 m), abiertos al crucero con arcos altos (fase 2). Las ventanas de los testeros (E21) tienen el alféizar a 4 m;
   - coro + ábside: una sola bóveda que sube desde todos los muros, con arranque a 11 m y clave a 16 m. El vitral (tope a 10 m) queda debajo del arranque.
5. **Velas flotantes.** Los grupos del spec en `x=0, y=41/47 m, z=5.2 m` no tienen soporte. Se reemplazan por **dos candelabros de pie a los lados de los novios** (`x=±2.4 m, y=47 m`).
6. **Esqueletos.** Tres props extra de 5 cm en el punto de la ceremonia `(0, 47, 0) m`: novia, novio y oficiante, como en la escena actual. Total: 15 de 20 props.
7. **Candelabros de pie de la nave.** Diez, en `x=±4.8 m`, en el centro de cada tramo excepto el del crucero. Son un solo diseño de prop repetido, una luz cada uno.
8. **Ignorado por el motor:**
   - la rugosidad del vidrio (el vidrio es nítido);
   - el radio de caída de las luces (se usa inverso del cuadrado y se ajusta a ojo).

### Luces resultantes (38; 23 desde la fase 7)

Cada luz sale de la llama de un prop (fase 5):

| Grupo | Cantidad |
|---|---:|
| Nártex (candelabros de pie) | 2 |
| Naves laterales (candelabros de pie, uno por tramo salvo el del crucero) | 10 |
| Lámparas colgantes | 6 |
| Brazos del crucero (candelabros de pie) | 2 |
| Coro (candelabros de pie) | 2 |
| Altar (cirio pascual + 2 candeleros) | 3 |
| Candelabros de pie de la nave | 10 |
| Candelabros junto a los novios | 2 |
| Luna | 1 |
| **Total** | **38** |

---

## 4. Notas del motor

- **Coordenadas.**
  - El spec usa `x` ancho (negativo al norte), `y` hacia el este y `z` hacia arriba.
  - El grid tiene Y hacia arriba: `grid = (x + 70, z + 2, y)` en vóxeles.
  - `scene/cathedral.rs` escribe todo en coordenadas del spec y convierte con `cell()`/`world()`.
- **Escala.** 1 unidad de mundo = 1 vóxel de 20 cm (`units_per_meter = 5`). Los props van en grids de detalle de 0.25 unidades (5 cm), como los esqueletos.
- **Tamaño.** Grid de 140 × 114 × 220 ≈ 3.5 M celdas (≈ 3.5 MB), después de la revisión de la fase 7.
- **Medición de la fase 0.** Cascarón completo con las 40 luces:

  | Resolución | Nave | Boda | Vitral | Crucero |
  |---|---:|---:|---:|---:|
  | 640×360 (cuadro interactivo) | 0.2 s | 0.2 s | 0.1 s | 0.1 s |
  | 1280×720 | 0.8 s | 0.7 s | 0.5 s | 0.5 s |

  Es el mismo costo que la iglesia original.
- **Salto de espacio vacío: probado y descartado.** Con bloques de 8³ fue más lento y con 16³ empató; el DDA ya es barato y el costo está en el sombreado.
- **La luz de colores está implementada.** La sombra a través del vidrio se tiñe con el color del vidrio, así el vitral pinta el piso.
- **Cómo se construye el cascarón.** El interior es una lista de salas (rectángulos + el polígono del ábside), cada una con su bóveda: cañón apuntado a lo largo de un eje, o bóveda que sube desde todos los muros. Se pone piedra a menos de un muro (0.8 m) de cada sala, hasta su clave + cascarón, y después se vacía cada sala. Así los muros y su altura salen solos, y nada queda abierto al exterior salvo los vanos tallados a propósito.
- **Ejecutar:** `cargo run --release` abre la catedral; `cargo run --release -- --church` abre la iglesia original (se puede borrar cuando ya no haga falta).

### Fases

0. ✅ Preparación: este documento, escala de 20 cm, cascarón y medición, luz de colores por el vidrio, luna.
1. ✅ Cascarón real: pisos, muros por zona, nártex, vestíbulos, crucero, coro, ábside, aberturas, puertas. La nave y las naves laterales todavía están separadas por el muro de la arquería, que se abre en la fase 2.
2. ✅ Tramo modular ×6: pilares, arquería, triforio ciego, claristorio, bóvedas de crucería, bóvedas laterales, crucero.
   - **Bóveda de la nave:** crucería cuatripartita hecha como la unión de dos cañones apuntados, uno transversal (luz de 12 m) y uno por tramo (8 m), ambos con arranque a 16 m y clave a 22 m. Así quedan lunetos sobre cada ventana del claristorio y aristas diagonales. Nervios de piedra clara de 0.8 m (fajones, diagonales y espinazo), con una clave que cuelga 0.4 m más, sobre una plementería oscura.
   - **Naves laterales:** el mismo esquema, con nervios de 0.6 m.
   - **Pilares compuestos:** 1.6 m con las esquinas achaflanadas, basa oscura y capitel a la altura del arranque de los arcos.
   - **Pilastras de 0.4 m** en el muro exterior de cada nave lateral, en cada límite de tramo.
   - **Triforio ciego:** dos nichos de 2.4 m por tramo.
   - **Tramo del crucero:** un arco de 6.4 × 14.4 m hacia cada brazo, en vez de la arquería.
3. ✅ Vitral (patrón 16×26 con emplomado grueso) y rosetón abierto. El rosetón quedó hecho en la fase 1.
   - **Vitral:** borde de rubí de 2 vóxeles, un anillo de plomo (hierro M09) de 1 vóxel y un campo de cobalto con una cruz de ámbar de centro pálido y cuatro bloques pálidos. La luna del este lo proyecta en colores sobre el punto de la ceremonia.
   - **Desvío del spec:** no hay plomo alrededor de cada motivo. En 16 columnas, esos anillos se comían todo el campo azul.
   - **Materiales nuevos:** vidrio cobalto, rubí, ámbar y pálido (transparencia 0.72, IOR 1.5, textura moteada sin líneas por celda) y hierro oscuro reflectivo.
4. ✅ Materiales: texturas de mármol M02/M03/M04, colores de vidrio, emplomado.
   - **Textura de mármol** (`Texture::marble`): tono con variación suave y vetas finas onduladas. Es periódica, así que se repite cada 1.6 m sin costura y sin contorno por bloque.
   - **Mármol pálido M02** (la Piedra de la rúbrica): muros, pilares, nervios y pilastras.
   - **Gris azulado M03:** plementería, fondos de nicho y basas.
   - **Molduras M04:** capiteles y aro del rosetón.
   - **Piso:** damero de baldosas de 1 m, pálido y gris, pulido (reflejo 0.12). Cuesta ~+30 % en las vistas con mucho piso a resolución completa; el cuadro interactivo sigue en 0.2 s.
   - **Herrajes de las puertas:** hierro M09.
5. ✅ Props de 5 cm: los 12 del spec, los esqueletos y los candelabros de pie.
   - **Cómo se hacen** (`scene/props.rs`): cada diseño es una función que arma su propio grid de 5 cm, ajustado al prop. Los que tienen velas devuelven la posición de su luz, justo encima de la llama. Así toda luz tiene su vela visible, y un test verifica que ninguna quede adentro de algo sólido.
   - **Ceremonia:** el spec pone la cruz procesional justo donde los novios se toman de la mano. De oeste a este queda así:
     - los novios frente a frente (y = 45.6 m);
     - los reclinatorios (46.4 m);
     - el oficiante mirando al oeste (47.8 m);
     - la cruz procesional detrás de él (48.8 m).
   - **Velas de muro → candelabros de pie.** Los grupos del spec en muros (naves laterales, nártex, coro, brazos del crucero) son candelabros de pie de un solo diseño. Los del coro van en x = ±4.6 m, no ±4, para no chocar con el ambón.
   - **Lámparas colgantes:** aro de hierro con 8 velas a 12 m, colgado de la clave de cada tramo con un tirante de hierro de 0.4 m (E30).
   - **Luces:** 37 velas (3 del altar, 30 candelabros, 6 lámparas) + la luna.
   - **Rendimiento:**
     - Probado y descartado: un grid grande compartido por varios props (los rayos recorren miles de celdas vacías de 5 cm) y ~55 grids sueltos (cada rayo, también cada rayo de sombra, los prueba a todos). Ambos duplicaban o triplicaban el render.
     - Lo que quedó: grids ajustados, agrupados por cercanía detrás de una caja envolvente (`group.rs`). Además, los rayos de sombra cortan en el primer obstáculo opaco en vez de buscar el más cercano.
     - Resultado a 1280×720: nave 1.0 s, boda 1.8 s. La boda es la vista más cara, por el detalle fino de los esqueletos, que también pagaba la iglesia original.
   - **`cargo run` abre la catedral**; `--church` abre la iglesia original.
6. ✅ Luces, cámaras y ajuste de exposición contra las imágenes de referencia.
   - **Umbral de luz** (`LIGHT_CUTOFF`): de 0.015 a 0.04, y ahora se resta en vez de solo cortar. Cada vela se apaga suave hasta 0 justo donde deja de evaluarse, sin borde visible. Se ahorran los rayos de sombra hacia velas lejanas que casi no aportan: la boda pasa de 1.9 s a 0.9 s a 1280×720, y el cuadro interactivo queda en 0.1–0.2 s en todas las vistas (unos 5–10 cuadros por segundo caminando).
   - **Exposición** de 1.4 a 1.8, para compensar el relleno que daban las velas lejanas. El resultado se parece más a la referencia: charcos cálidos de vela y sombras azuladas de luna entre ellos.
   - **Luna:** intensidad de 1.3 a 2.0, para que el charco de colores del vitral se lea bien sobre el damero.
   - **Cámaras:** la vista del rosetón apunta a su centro. Vista nueva **"altar"**: detrás de los novios a la altura de los ojos, mirando al vitral, como la imagen de referencia.
   - **Iglesia original:** usa el mismo motor, así que también cambió de exposición y quedó más rápida (interior 1.2 s → 0.3 s a 1280×720).
7. ✅ Revisión del autor: iglesia más corta, naves proporcionadas y un piso con significado. **Manda sobre los números anteriores de largo, ancho y posiciones.**
   - **Largo: de 68 m a 44 m.** Se sacaron 2 tramos de nave antes del crucero y el tramo después del crucero. La planta queda:

     | Zona | y (m) |
     |---|---|
     | Nártex | 0–8 |
     | 2 tramos de nave | 8–24 |
     | Crucero | 24–32 |
     | Coro | 32–36 |
     | Ábside | 36–44 |

     Todo sale de pocas constantes en `cathedral.rs` (`NAVE_BAYS`, `NAVE_HALF`…).
   - **Ancho: la nave pasa de 12 m a 8.8 m** (7.2 m entre pilares) y cada nave lateral de 2.4 m a 4 m. Es la proporción gótica de 2 : 1, con el mismo ancho total: solo se corrió la arquería hacia adentro. El coro y el ábside toman el ancho de la nave.
   - **Luna a 25°**, para que con el ábside más cerca su luz siga cayendo en el centro del crucero.
   - **Piso.** Se va el damero de alto contraste, que a lo lejos se volvía moiré, sobre todo en la vista previa en movimiento. El piso se lee de la puerta al altar, como en una iglesia real:

     | Zona | Dibujo | Significado |
     |---|---|---|
     | Nártex, naves laterales, brazos, vestíbulos | mármol pálido liso | el mundo cotidiano |
     | Límites de tramo y arquerías | bandas beige | el piso repite la bóveda que tiene encima |
     | Nave | alfombra procesional de 3.2 m (borde oscuro, campo pálido) con un rombo oscuro de centro dorado por tramo | el camino del peregrino y sus estaciones |
     | Crucero | sol dorado de 12 rayos, Ø 5.6 m, uno apunta al altar | el "Sol de justicia" (Malaquías 4:2); los doce apóstoles o meses |
     | Umbral del coro | banda dorada | el límite entre la nave y el presbiterio |
     | Presbiterio | campo oscuro con borde pálido | lo sagrado, apartado; hace resaltar el altar y el vitral |

   - **Los novios están parados en el centro del sol,** y la luna cae sobre ellos por el vitral: sol y luna, la imagen antigua del matrimonio, se encuentran en la boda.
   - **Material nuevo:** mármol amarillo pulido ("giallo antico") para el sol y los acentos dorados. El beige de las bandas es el mármol de molduras, pulido.
   - **Posiciones nuevas:**
     - presbiterio: el del spec, corrido 24 m al oeste;
     - ceremonia: centrada en el sol (y = 28 m), con reclinatorios, oficiante y cruz al este;
     - candelabros de la nave: en x = ±3.6 m;
     - candelabros junto a los novios: al borde del sol, x = ±3.2 m;
     - candelabros del coro: en (±3.6, 35.2) m;
     - púlpito: en y = 22 m.
   - **Columnas de la nave** (reemplazan los pilares cuadrados). De abajo hacia arriba:
     - basa: plinto oscuro de 2 m y el bloque original de 1.6 m achaflanado, hasta 0.8 m;
     - anillo de transición;
     - fuste cilíndrico esbelto de ~1.3 m;
     - capitel redondo y ábaco cuadrado, del que arrancan los arcos (5.6 m).

     Encima, un fuste adosado de media caña sube por la cara de la nave hasta el arranque de la bóveda (16 m), donde apoyan los nervios. Las columnas de los extremos quedan como medias columnas embebidas en el muro del nártex y en el del coro.
   - **Detalle en las columnas**, inspirado en los paneles de mármol de color enmarcados en blanco de las iglesias de referencia, pero en gris. El rosado se probó y se descartó: se veía fuera de lugar; el autor pidió algo más gótico y sobrio.
     - fuste de mármol gris azulado veteado, un "bardiglio" (material nuevo), con dos anillos pálidos y capitel de mármol pálido (el dorado se probó y se descartó: más simple así);
     - fuste adosado también gris, con un anillo pálido y un remate pálido donde apoyan los nervios;
     - cada pilastra de las naves laterales lleva un panel gris enmarcado en pálido.
   - **Ábside semicircular** en vez del octogonal. El coro se alarga a 6.8 m y el ábside tiene 5.2 m de radio, así el largo total sigue en 44 m. Coro y ábside van cubiertos por un cañón de medio punto de 8.8 m de luz (arranque a 11.6 m, clave a 16 m), en mármol gris azulado oscuro como la plementería. Visto desde la nave, su boca es un gran **arco triunfal** redondo, enmarcado por una moldura pálida. Donde el ábside se angosta, el mismo cañón baja y forma la media cúpula. La primera versión usaba la bóveda que sube desde todos los muros, y eso dejaba una línea recta horizontal a 11 m en la entrada del coro. El vitral atraviesa el muro curvo en el eje. Los vestíbulos, que usan el mismo tipo de bóveda, también quedaron oscuros.
   - **Naves laterales más redondas y livianas:**
     - la crucería es de medio punto (4 m de luz, arranque a 9.6 m, clave a 11.6 m) en vez de un apuntado aplastado;
     - nervios de 0.4 m en vez de 0.6 m;
     - las pilastras suben hasta el nuevo arranque;
     - los arcos de la arquería tienen dos órdenes: un arco 0.2 m más grande entra 0.2 m en cada cara, así el borde se lee como dos escalones finos y no como un muro de 0.8 m cortado.
   - **Contenido de iglesia real en los espacios vacíos:**
     - **Brazos del crucero:** cada testero es una capilla lateral, con un altar lateral de mármol y roble (prop nuevo) y encima un panel oscuro enmarcado con una cruz dorada en relieve. A cada lado hay un soporte de velas votivas (prop nuevo: pirámide de hierro con filas de velitas, una luz por soporte), que reemplaza al par de candelabros de los brazos. La ventana del testero sube a 6 m de alféizar para dejarle lugar al panel.
     - **Bolsillos al final de las naves laterales**, junto al coro (entre el brazo del crucero y el muro del fondo, 0.8 m de hondo): un panel oscuro enmarcado con un relieve, el **sol** dorado al sur y la **luna** creciente pálida con estrellas al norte, como flanquean la cruz en el arte medieval. Retoman el sol del piso y la luna del vitral.
     - Se probaron santos en relieve en los nichos del triforio y en los paneles de las capillas, y se descartaron: al autor no le gustó la figura.
   - **Dibujos en las columnas al estilo de la catedral de Durham,** solo en las dos caras del fuste que se ven al recorrer la iglesia: la que mira a la entrada y la que mira al altar. No van tallados: son líneas de mármol pálido sobre el gris, como dibujadas. El dibujo alterna de columna en columna: chevrones, espirales, rombos.
   - **Novios y oficiante como personas** (`scene/people.rs`, que reusa el `Body` de los esqueletos):
     - novio de traje oscuro con camisa blanca y moño;
     - novia con vestido blanco de falda acampanada, velo semitransparente, pelo recogido y ramo;
     - oficiante con alba blanca, estola roja y el libro.

     Materiales nuevos: piel, pelo y tela blanca. Siguen frente a frente, tomados de la mano sobre el sol.
   - **Cámara nueva "sol":** desde el crucero hacia el relieve del sol.
   - **Luces:** 20 velas (3 del altar, 10 candelabros, 4 soportes votivos, 3 lámparas colgantes) + la luna.
   - **Cámara nueva "lateral":** por la nave lateral norte, mirando al este.
   - **Luces:** 18 velas + la luna. Se sacaron los candelabros junto a los muros de las naves laterales, que quedan en penumbra con luz de luna. La luz de las velas pasó de (255, 168, 88) a un ámbar más cálido, (255, 136, 52); la luna sigue fría.
   - **Cámara nueva "planta":** casi en picada desde 20 m, para revisar el piso.
   - **Tiempos:** cuadro interactivo de 0.1–0.2 s en todas las vistas.

8. **Invasión zombi ✅** (`scene/zombies.rs` + `invasion()` en `cathedral.rs`). La boda, congelada a mitad del asalto: nada se mueve, todo queda detenido en pleno movimiento.
   - **Decisiones del autor:**
     - la puerta principal está rota; las laterales siguen cerradas;
     - el vitral del ábside primero resistía; en la segunda ronda el autor lo quiso más grande y reventado, lleno de zombis;
     - gore completo;
     - se acepta un render un poco más lento (al final no hizo falta).
   - **Cómo se arma un zombi:** se estampan unas pocas "cápsulas" (un segmento con radio) en celdas de 5 cm: torso, cabeza con cuencas oscuras y la mandíbula colgando ensangrentada, brazo, antebrazo, mano, muslo y pierna. Cada pose sale de la dirección de cada tramo. Hay tres poses:
     - `wild_pose`: para las masas, trepándose unos sobre otros, de cara a la iglesia;
     - `walking`: paso rengo, inclinado, con los brazos estirados;
     - `climbing`: cabeza abajo, con el pecho contra la columna o el muro.

     Lo feo sale de la pintura, en manchas de 20 cm:
     - piel de cadáver gris con moretones (material nuevo);
     - ropa de un solo color por prenda, sacada de harapos oscuros, negro o camisa sucia (material nuevo), con agujeros y sangre;
     - sangre húmeda con algo de brillo (material nuevo);
     - hueso a la vista, que es el Hueso con musgo de la rúbrica.

     La escala es la de los novios. La primera versión se leía como **follaje**. Las causas eran un moteado fino de 10 cm, piel verdosa y miembros en direcciones al azar. Se corrigió con:
     - caras visibles;
     - una prenda de un solo color por cuerpo;
     - manchas más grandes y más escasas;
     - piel gris en vez de verde;
     - poses más ordenadas.
   - **Masas en cada abertura** (`zombies::mass`). Cada una tiene tres partes:
     - un **tapón** que llena el vano: cuerpos enteros empujando hacia adentro, con la cabeza por delante, y detrás un relleno apretado de torsos y restos para que se vea una pared de carne y no una maraña con cielo;
     - una **cascada** que baja por el muro, cabeza abajo, más rala hacia abajo;
     - en la puerta y en el ábside, una **montaña** que baja hasta el piso.

     | Abertura | Cuerpos | Detalle |
     |---|---|---|
     | 6 ventanas altas de la nave | 130 c/u | cascada de 4 m, chorreaduras de sangre bajo cada una |
     | Rosetón (reventado) | 160 | cascada de 5 m que se une con la montaña de la puerta |
     | Puerta principal (rota, vano de 4 × 8 m que atraviesa el muro) | 220 | montaña de 3.2 m sobre el nártex, con las hojas rotas (roble y herrajes) entre los cuerpos y un charco |
     | 2 ventanas de los testeros del crucero | 130 c/u | cascada de 2 m sobre las capillas, chorreaduras y charco |
     | Vitral del ábside (reventado) | 180 | montaña de 3 m detrás del retablo, astillas de vidrio en el piso, sin relleno y con un hueco por donde sigue entrando la luna |
   - **Los que se separaron**, cada uno por su cuenta (40 en total):
     - bajando cabeza abajo por las columnas: dos en cada una de las 4 columnas exentas, a distintas alturas y del lado que se ve;
     - uno en cada fuste adosado alto de la nave;
     - dos más por los muros altos;
     - 8 cayendo desde las ventanas altas, a distintas alturas, con los brazos y las piernas sueltos;
     - 3 que ya se estrellaron contra el piso de la nave, con la sangre saltando del golpe (ver abajo);
     - 13 caminando hacia los novios por el nártex, la nave, las naves laterales y los brazos del crucero;
     - uno arrastrándose por la nave, dejando un rastro de sangre.
   - **La mordida como el beso de una boda** (`zombies::dip` + `people::dipped_bride`). El zombi tiene a la novia inclinada hacia atrás, como el novio que la inclina para besarla, pero le muerde la garganta.
     - La novia se arma con cápsulas y conos, porque inclinada no entra en cajas alineadas a los ejes. Tiene la falda acampanada hasta el piso, una rodilla levantada bajo la tela y el torso a 70° de la vertical. La cabeza cuelga hacia atrás, gritando. El velo cae hasta el piso, un brazo cuelga suelto y el otro se aferra al zombi. El ramo quedó tirado, con pétalos.
     - El zombi se arma desde la herida: la punta de su mandíbula va en la garganta. Encorvado sobre ella, pasa un brazo por debajo de su espalda y con el otro la agarra de la cintura, en estocada. Las piernas apoyan en el piso con una IK de dos tramos (`leg_to`).
     - La sangre baja por el corpiño hacia la cintura, que queda más abajo que el cuello, y gotea de la garganta al piso en tramos cortados, donde salpica.
     - El canvas de 5 cm de las cápsulas se movió a `scene/canvas.rs`, porque ahora lo usan los zombis y la novia.
   - **Salpicaduras de impacto** (`zombies::splash`), congeladas en el instante del golpe y no como un charco quieto:
     - una mancha central de borde irregular;
     - chorros en estrella que terminan en una gota;
     - una corona de puntas que se levantan alrededor, cada una con su gota suelta arriba;
     - unas 30 gotas todavía en el aire sobre parábolas, estiradas en la dirección en la que vuelan.

     Van bajo los 3 cuerpos estrellados (`zombies::impact`) y, en chico, donde gotea la garganta de la novia.
   - **La reacción** (`people.rs`), todos con la boca abierta en un grito:
     - el novio, afirmado con un pie adelante y otro atrás, agarra al zombi por la cintura y tira para sacárselo de encima;
     - el oficiante levanta las manos del espanto, y el libro abierto quedó tirado a sus pies.
   - **El vitral del ábside, más grande y reventado:**
     - es una lanceta de 4 × 8.4 m que arranca a 2.8 m, justo sobre el retablo;
     - solo quedan astillas pegadas al marco, de hondo desparejo;
     - la masa que entra por él tiene un hueco justo donde pasa el rayo de luna que va a la boda (entra a unos 8 m), y la montaña se queda más baja que ese rayo. Así la luna sigue iluminando la escena, y desde la vista "vitral" se ve la luna llena entre las siluetas de los cuerpos.
   - **Costo:** cada zombi tiene un grid ajustado a sus cápsulas. Con el margen fijo de 1.3 m que usaba antes, los 30 sueltos hacían el render 5 a 10 veces más lento. Van en 9 `Group`: las masas norte, sur, oeste, de los brazos y del ábside, y los sueltos en cuatro cuartos, al norte o al sur y antes o después del crucero. Cuadro interactivo de 0.1–0.2 s. A 1280×720 con supermuestreo tarda entre 1.2 y 4 s según la vista; las más lentas son las de la boda. La escena se construye en 0.14 s.
   - **Cambios de lugar:** la cámara "nave" y el punto de partida de la primera persona se corren a y = 6.8 m, delante de la montaña de la puerta.
   - **Cámaras nuevas:**
     - "puerta": la montaña bajo el rosetón;
     - "mordida": la novia inclinada, de cerca.

     "boda" y "mordida" miran desde el lado del ábside, porque la luna que entra por el vitral roto ilumina esa cara de la escena. Desde la nave, la boda queda a contraluz.

9. **Más rápida, más oscura y zombis radiactivos ✅**
   - **Rendimiento.** Se sabe por qué se sentía lenta. Al quedarse quieta la cámara, la ventana renderizaba la calidad completa (1280×720) de una sola vez, hasta 1 s, y durante ese tiempo no atendía el teclado ni el mouse. Arreglos:
     - **Calidad completa progresiva** (`render_rows`): se hace de a franjas de 90 filas, una por vuelta del bucle, y si la cámara se mueve se abandona.
     - **Saltear lo que está detrás.** Cada objeto informa a qué distancia entra el rayo a su caja (`RayIntersect::entry_distance`). Un grupo cuya caja empieza más lejos que el impacto ya encontrado no se recorre: la catedral va primero, y los zombis detrás de un muro se descartan con una prueba de caja. Vale igual para los rayos de sombra, con la distancia a la luz, y adentro de cada `Group`.
     - **Luces más débiles** (abajo). Con el umbral de corte, cada vela alcanza menos distancia y cada punto evalúa menos rayos de sombra.

     | | Antes | Después |
     |---|---|---|
     | Vista previa (640×360) | 0.1–0.3 s | 0.1 s |
     | Calidad completa (1280×720) | 0.3–1.0 s, bloqueando | 0.2–0.5 s, interrumpible |
   - **Más oscura:**
     - todas las velas bajan a ~55% (candelabros de la nave 38, pares 33/28, altar y votivas 22, lámparas colgantes 140);
     - la luna baja de 2.0 a 1.1;
     - el ambiente baja de (24, 28, 42) a (12, 14, 24). Es una constante compartida, así que la iglesia original también quedó más oscura.
   - **Zombis radiactivos:**
     - piel verde podrida con un resplandor propio tenue: el campo nuevo `glow` del material, que se suma a la iluminación, a diferencia de `emission`, que la reemplaza (las llamas);
     - ojos verdes que brillan. Se sacaron en la fase 10: en las masas al azar parecían las luces de un árbol de Navidad.

10. **Pirámides humanas, carne podrida y un haz de luna sobre la boda ✅**
    - **Las masas eran al azar**: cientos de cuerpos tirados en una zona con poses al azar. Por eso se leían como follaje, y en la ventana, con los ojos brillantes, como un árbol de Navidad. Ahora son **pirámides humanas armadas cuerpo por cuerpo** (`zombies::pyramid`):
      - **en el piso** (puerta y ábside): pisos de zombis en cuatro patas contra el muro. Cada piso se apoya en las espaldas del de abajo y tiene una fila menos adelante y un cuerpo menos a los costados. En la cima, algunos de pie se estiran hacia arriba;
      - **en el vano**: una pila de filas que se achica hacia arriba (4-3-2-1), con la de adelante asomada sobre el alféizar;
      - **en la puerta**, además, una fila de pie apretada en el vano roto.

      Solo se estampan los cuerpos que se ven, el frente y los costados de cada piso. Las poses nuevas son `crawling` (en cuatro patas, con la cara levantada) y `reaching_up`.
    - **Los que se separaron** (pedido del autor: que en los bordes cada zombi se identifique, porque salió de la pila): van bien espaciados y aparte de cada pirámide. En las ventanas bajan por el muro cabeza abajo, escalonados; en el piso caminan o se arrastran desde los costados y el frente, rumbo a la nave.

      | Abertura | Pirámide | Sueltos |
      |---|---|---|
      | Puerta | 4 pisos, base de 7, cima de 3 de pie, fila de 11 en el vano | 6 |
      | Vitral del ábside | 3 pisos detrás del retablo, base de 6; pila de 3 filas en el alféizar, con lo alto abierto para la luna | 4 |
      | Rosetón | pila de 6 filas, recortada al círculo | 4 bajando hacia la puerta |
      | 6 ventanas altas | pila 4-3-2-1 | 2 c/u |
      | 2 ventanas del crucero | pila de 5 filas | 3 c/u |
    - **La cara hacia donde van** (`Pose::look`): los ojos y la mandíbula ya no siguen al pecho. Los que gatean levantan la cara hacia la nave y los que bajan por los muros miran hacia abajo y hacia la iglesia, no contra la piedra.
    - **Desnudos, de carne podrida amarillo verdosa**: sin ropa, todo el cuerpo con la misma textura, con parches de sangre y hueso. El resplandor tenue sigue en la piel. Los ojos ya no brillan: son cuencas oscuras.
    - **La sangre resplandecía un poco** (`glow` 0.45); en la fase 11 el autor pidió sacarlo.
    - **Un haz de luna sobre la boda** (`Light::spot`, un foco con cono de borde suave; afuera del cono no evalúa ni el rayo de sombra). Es frío, azul pálido, y baja desde 18 m sobre el crucero, corrido para que la lámpara colgante no lo tape. Deja un charco de ~2 m de radio sobre los novios, que con la iglesia oscura se perdían.
    - **Tiempos:** vista previa de 0.1 s en casi todas las vistas y de 0.3 s en las de la boda. Allí la luna ahora entra por lo alto del vitral abierto y las pirámides dejan pasar los rayos, que antes chocaban contra masas macizas. La escena se construye en 0.07 s.

11. **Montones en la puerta y el ábside, más sangre y personas que se notan ✅**
    - **En la puerta y en el vitral del ábside vuelve el montón al azar** (`zombies::heap`), que al autor le gustaba más ahí que la pirámide. Se mantienen el cuerpo desnudo de carne podrida y la mandíbula ensangrentada; el relleno del tapón también es carne.

      | Abertura | Montón |
      |---|---|
      | Puerta | 220 cuerpos, montaña de 3.2 m, 6.4 m de alto contra el muro, relleno detrás del tapón, hojas rotas |
      | Vitral del ábside | 180 cuerpos, montaña de 3 m, 6 m de alto, sin relleno y con el hueco por donde entra la luna, astillas |

      Las pirámides quedan en las ventanas (`zombies::pyramid`, solo la pila en el vano y los que bajan por el muro). Se sacó el código de las pirámides del piso, que ya no usaba ninguna abertura. Las hojas y las astillas quedaron en un solo lugar (`Wreckage`).
    - **Más sangre, sobre todo en la entrada:**
      - un charco de 2.4 m de radio bajo el montón de la puerta y cinco más alrededor;
      - cuatro rastros de arrastre desde la puerta hacia la nave (`blood_trail`: manchas cada 60 cm con un zigzag, que se afinan);
      - 30 chorreaduras por la cara interior de la fachada y 12 más bajo el rosetón (`blood_drips` ahora sirve para muros normales a `x` o a `y`);
      - cinco salpicaduras frescas en el nártex, con las gotas en el aire (`zombies::splatter`), lejos del punto de partida de la primera persona.

      En el resto de la iglesia hay nueve charcos más (al pie de las columnas con zombis, en la nave y en las naves laterales) y más chorreaduras bajo las ventanas altas (10 por ventana) y las del crucero (12).
    - **La sangre ya no resplandece.**
    - **La piel de los novios y del oficiante brilla muy poco** (`glow` 0.1), para que se noten en la iglesia oscura.
    - **Tiempos:** vista previa de 0.1–0.2 s y calidad completa de 0.2–0.8 s.

12. **Vitrales en las naves laterales ✅**
    - **8 lancetas altas** en los muros de las naves laterales (`aisle_windows`): de a dos por tramo, 4 por lado, de 1.6 × 5 m con el alféizar a 1.6 m, entre las pilastras.
    - **Vidrio emplomado liso**, sin motivo, como pidió el autor: rombos de cobalto, rubí, ámbar y pálido mezclados entre plomos de hierro, con un marco de hierro.
    - **Están enteros, sin zombis.** La luna viene del este y no llega a estos muros, así que el vidrio tiene un resplandor propio tenue (`glow` 0.9 sobre los mismos vidrios del ábside) para que se vea de noche.
    - El perfil apuntado del vano (`lancet_columns`) lo comparten ahora este vitral y el del ábside.
    - **Las ventanas del crucero vuelven al montón al azar:** 130 cuerpos, tapón con relleno y una cascada de 2 m por el muro (`Heap::spill`). Las pirámides quedan en las ventanas altas y en el rosetón.
    - **La sangre vuelve a resplandecer, casi imperceptible** (`glow` 0.1).
    - **Tiempos:** vista previa de 0.1–0.2 s.

13. **Ajustes: una lanceta por tramo, zombis saliendo por el marco del ábside, la mordida más clara ✅**
    - **Naves laterales:** una sola lanceta grande por tramo en vez de dos, más alta (2.4 × 7.2 m, alféizar a 1.6 m), centrada entre las pilastras. Son 4 en total.
    - **El vitral del ábside, como el dibujo del autor:**
      - el montón de siempre (140 cuerpos) solo en la mitad de abajo del vano, con `Heap::plug` como alto del tapón (reemplaza al hueco para la luna);
      - arriba, **siete zombis que salen trepando por el marco** (`zombies::rim`), repartidos por el contorno apuntado desde media altura de una jamba, por el arco, hasta la otra. Cada uno tiene la cabeza en el borde y el cuerpo afuera, sobre el muro, con las manos enganchadas en el canto, las piernas abiertas contra la piedra y la cara vuelta hacia la iglesia. El de la clave cuelga de la punta con los dos brazos, asomándose;
      - lo alto del vano sigue abierto: se ve la luna entre ellos.
    - **La mordida, más clara.** En la captura del autor, vista desde la nave, era una mancha oscura:
      - el haz de luna ahora baja del lado de la nave (a 14 m, 6 m antes de los novios), así los ilumina de frente para quien llega caminando;
      - el velo cae medio metro en vez de llegar al piso: era una columna translúcida que tapaba la cabeza de la novia desde los costados;
      - las cámaras "boda" y "mordida" vuelven al lado de la nave; "mordida" muestra el perfil de la novia inclinada.

14. **El vitral del ábside revienta con sentido físico, y cabezas cuadradas en la mordida ✅**
    - **Se sacaron los que trepaban por el contorno** (`rim`): flotaban delante del muro sin tocar el borde, y arriba formaban una pirámide rara. Ahora cada uno está sostenido por algo (`zombies::breakout`):
      - dos colgados de las jambas rotas, con las dos manos enganchadas en el canto del vano, una sobre otra (los brazos llegan con la IK de dos tramos). El cuerpo cuelga contra el muro, al costado del vano, con las piernas en el aire y la cara hacia la iglesia;
      - dos parados en lo alto del montón, arrodillados, tirándose afuera a través del vidrio con los brazos estirados.
    - **El vidrio en el instante en que revienta:** 140 astillas de todos los colores del vitral congeladas en el aire. Salen del vano y siguen su parábola hacia adentro, cayendo hacia el altar. Tienen el resplandor tenue de los vitrales laterales, porque sin él se veían negras contra la noche.
    - **Lo que queda en el marco, más roto:** astillas de 1 a 7 vóxeles de hondo en trozos de 2 × 4, y huecos en el borde de rubí donde se rompió hasta la piedra.
    - **Cabezas cuadradas y grandes** para la novia y el zombi que la muerde, como las del novio y el oficiante (25–26 cm). Son cajas torcidas (`Canvas::cuboid`), con el pelo arriba y atrás, ojos oscuros y la boca abierta. La del zombi tiene la mandíbula ensangrentada contra la garganta.
    - **Tiempos:** vista previa de 0.1–0.2 s.

15. **Los que caen salen de las ventanas altas; astillas grandes ✅**
    - **Caen de las ventanas altas, y se ve de dónde** (`zombies::falling_from`). En cada una de las 6, uno se vuelca por encima del alféizar, cabeza abajo, con la cadera todavía en el borde. Otros uno o dos ya están en el aire, más abajo y más adentro cuanto más cayeron (la parábola de quien sale empujado del vano). En total son 15 y reemplazan a los 8 puestos a mano. Los 3 estrellados en el piso siguen debajo.
    - **Las ventanas altas tienen vidrio roto:** astillas pegadas al marco, de hasta 3 vóxeles de hondo. El código del vitral reventado es uno solo (`shattered_pane`) para el ábside y para estas ventanas. Además, 14 astillas vuelan alrededor de los que caen.
    - **Astillas grandes, no papel picado** (`zombies::shard_burst`, `Canvas::triangle`): triángulos planos e irregulares de 10 a 28 cm, girados al azar y de un solo color cada uno (55% cobalto, 25% rubí, el resto ámbar y pálido). En el ábside son 45, antes 140 palitos.
    - **Se sacó el ramo.** El novio ya no tira del zombi: tiene las dos manos arriba, a los costados de la cabeza, casi agarrándosela del espanto.

16. **Nichos claros, el crucero revienta con sentido físico, vitrales que se rajan ✅**
    - **Los nichos del triforio ya no parecen ventanas.** Tenían el fondo de mármol oscuro y el autor los confundía con vanos. Ahora el fondo es de mármol gris, como los paneles de las columnas, con **tracería pintada** en mármol pálido (`painted_tracery`, sin relieve): un filete por el borde, un parteluz al medio que sube hasta un óculo en lo alto del arco, y puntos en rombo en las dos luces. Se leen como ventanas ciegas dibujadas.
    - **Las ventanas del crucero, junto al sol y la luna, revientan como el vitral del ábside:**
      - vidrio roto en el marco (`shattered_pane`, hasta 7 vóxeles de hondo);
      - el montón llena la mitad de abajo del vano (tapón de 3.5 m), con cuerpos colgando del alféizar;
      - uno se vuelca por el borde y dos caen (`falling_from`); dos ya se estrellaron en el piso de la capilla, delante del altar (`impact`);
      - dos cuelgan de las jambas y dos se tiran afuera desde lo alto del montón (`breakout`);
      - **las astillas salen disparadas lejos**: 40, a 5–13 m/s, cruzando el brazo hacia el crucero, casi hasta la escena de la boda.

      `breakout` ahora recibe cuántas astillas y a qué velocidad: en el ábside siguen siendo 45 a 2–4.5 m/s.
    - **Los vitrales de las naves laterales empiezan a rajarse** (`glass_cracks`): desde un punto de golpe salen fisuras claras (vidrio pálido con brillo, la luz en la rajadura) que zigzaguean por los rombos. Dos están muy rajados, con 7 fisuras largas, un anillo de telaraña y un agujerito en el centro. Los otros dos tienen 3 fisuras cortas. Para que lo blanco sean solo las fisuras, los paños ya no llevan pálido: cobalto, rubí y ámbar.

17. **Las ventanas del crucero se vacían siguiendo la gravedad ✅**
    - El montón en la mitad de abajo del vano se leía como follaje: una sábana de cuerpos pegada al muro, no gente que acaba de entrar. Ahora es un **torrente** (`zombies::torrent`), en un solo instante:
      - **en el vano** quedan 4, trepando por encima del alféizar, asomados hacia abajo;
      - **en el aire** está la mayoría: 22, cada uno en un momento distinto de su caída (de 0 a ~0.9 s), empujado desde el borde a 1.2–2.8 m/s hacia adentro. Van sobre su parábola, se amontonan cerca de la ventana, donde todavía van lentos, y se abren al caer;
      - **en el piso**, 12 que ya cayeron, rotos y uno sobre otro donde terminan las parábolas (a 1.8–3.6 m del muro), con tres salpicaduras de sangre. Quedan lejos de las velas votivas.
    - Siguen los dos colgados de las jambas y las astillas disparadas lejos. Las piezas del ábside quedaron como funciones separadas: `hanging`, `lunging`, `shard_burst`.
    - **Las poses de caída son menos desordenadas** (desorden de 1.0 a 0.6): con los miembros en cualquier dirección parecían ramas.
    - **Cámara nueva "capilla":** la capilla del brazo norte, con su ventana.
    - El montón al azar (`Heap`) ya no tiene la cascada por el muro, que no usaba nadie.

18. **La gravedad también en el vitral del ábside y en la puerta ✅**
    - **El vitral detrás de los novios** usa el mismo torrente que las ventanas del crucero (`torrent`, ahora con parámetros): 4 en el alféizar, 16 cayendo y 14 rotos en el piso detrás del retablo, entre las astillas.
      - El alféizar está a 2.8 m, así que las caídas son cortas. Dónde caen sale de la misma parábola (empujados a 1.2–2.8 m/s): de 1.3 a 2.5 m del muro, sin llegar al retablo.
      - Con la gravedad honesta, todos los que entraron quedan por debajo del vano y la ventana se veía vacía. Por eso, **afuera, una montaña de 110 cuerpos** apoyada en el suelo del otro lado del muro sube hasta 3 m por encima del alféizar. Por el vano se ven trepando hacia adentro, recortados contra la luna.
      - Siguen los dos colgados de las jambas y las astillas en vuelo. Lo alto del vano queda abierto para la luna.
    - **La puerta: una estampida, no una caída** (`zombies::surge`). Nadie cae de alto; se aplastan:
      - una multitud apretada de 24 empuja en el vano y afuera;
      - 10 de adelante tropiezan y se van de boca, pivoteando sobre un pie, más inclinados cuanto más adentro, con los brazos para parar el golpe (`tripping`);
      - dos se tiran de cabeza por encima de los caídos;
      - 15 caídos boca abajo, pisoteados, algunos encima de otros, y 4 arrastrándose para salir de abajo;
      - salpicaduras donde golpean y las hojas rotas de la puerta entre los cuerpos.

      El montón queda bajo y se extiende hacia adentro, como se derrama una multitud. Reemplaza al montón de 6.4 m de alto contra el vano, que era lo menos físico de la escena.
    - Se sacaron `heap` y `lunging`, que ya no usaba nadie. Los cuerpos, las salpicaduras y lo tirado se estampan con una sola función (`finish`).

19. **Plata en vez de mármol oscuro ✅**
    - Los paños oscuros de las bóvedas se confundían con ventanas. Ahora son de **plata** (material nuevo `silver`): la textura ornamental del metal sobre un gris claro, con algo de reflejo que atrapa las velas y un brillo propio muy tenue. En la iglesia oscura, el metal sin luz se veía gris oscuro, no plateado.
    - **Lo que era del mismo mármol oscuro también pasó a plata:**
      - la plementería de las crucerías;
      - las cuencas de la media cúpula del ábside y de las cúpulas de los vestíbulos;
      - los paneles detrás de la cruz de las capillas y detrás del sol y la luna;
      - los plintos de las columnas.

      Los campos oscuros del piso (el presbiterio, los bordes de la alfombra) siguen de mármol.
    - **Tiempos:** vista previa de 0.1–0.2 s (el reflejo de la plata casi no se nota en el costo).

20. **Luz que atraviesa los vitrales rajados; sin sol ni luna en los paneles ✅**
    - **Luna por los vitrales de las naves laterales.** La luna viene del este y no llega a esos muros, así que cada uno de los 4 vitrales rajados tiene un foco afuera, 7 m afuera y 8 m más alto, que apunta a través del vidrio hacia el piso de la nave lateral (cono de 21°, frío, intensidad 6000).
      - La luz pasa teñida por cada rombo (la sombra con transmisión de color que ya tenía el motor) y blanca por los agujeritos de las rajaduras, así cada vitral pinta su dibujo en el piso y en los plintos, hacia la nave central.
      - Afuera del cono no se evalúa nada, así que el costo casi no cambió: vista previa de 0.1–0.2 s. Son 26 luces.
    - **Se sacaron el sol y la luna** de los paneles al final de las naves laterales (y las estrellas). Queda el panel de plata con su marco pálido (`aisle_end_panels`). La cámara "sol" pasó a llamarse "panel".

21. **Ambiente: bruma, resplandor, viñeta y gradación ✅**
    - **Bruma de profundidad** (en `cast_ray`): cada superficie se funde hacia un negro azulado frío según la distancia que recorrió el rayo, a la mitad a unos 25 m. Vale también para los reflejos. Los rayos que salen al cielo por una ventana no se tocan: la luna y las estrellas quedan nítidas en los vanos. El fondo de la nave y las bóvedas se hunden en la penumbra, y la iglesia se siente más profunda.
    - **Retoques sobre la imagen terminada** (`post.rs`, sin lanzar rayos):
      - **resplandor:** lo que pasa de un umbral de brillo (las llamas, el vidrio que brilla, las astillas, el piso bajo el haz de luna) se difumina (tres pasadas de caja en cada eje) y se suma en un halo suave;
      - **viñeta:** las esquinas se oscurecen hasta un 45%;
      - **gradación:** las sombras un poco hacia el azul y las luces un poco hacia el ámbar.

      Se aplican a la vista previa, a la calidad completa (al terminar la última franja) y a las capturas.
    - **Costo:** medido uno tras otro, entre 0.05 y 0.1 s más por cuadro de 1280×720 (~10%).

22. **Optimización sin cambios visibles ✅**
    - **Medición antes de tocar nada:** apagando una cosa por vez, en la vista "nave" (0.75–0.8 s por cuadro de 1280×720 con la máquina cargada), los rayos de sombra eran ~60% y los reflejos/refracciones ~35%. Por tipo de luz: la luna ~0.2 s, los 5 focos ~0.2 s, las 20 velas ~0.2 s.
    - **Arreglos, todos sin cambio visible** (diferencia media de ~0.03/255 por píxel contra el antes):
      - **los rayos de sombra se cortan en la luz** (`RayIntersect::ray_intersect_within`): el DDA del grid no sigue recorriendo más allá de la llama, y los grupos tampoco buscan más allá de lo ya encontrado;
      - **vanos para las luces de afuera** (`light::Aperture`): la luna viene del este y adentro solo entra por el vitral del ábside; los focos de las naves laterales, solo por su vitral. Un punto cuya línea hacia la luz no cruza el vano ni lanza su rayo de sombra. Antes, todo el piso lanzaba un rayo largo hacia la luna que terminaba en la piedra;
      - **rayos secundarios que no se notan no se lanzan** (`MIN_RAY_WEIGHT` = 3%): el reflejo de un reflejo (12% × 12%) o el reflejo de frente en un vidrio (~4%);
      - **DDA más liviano:** el índice de la celda se lleva sumando, y el borde del grid se revisa solo en el eje que avanzó;
      - **los retoques de la imagen** (resplandor, viñeta, gradación) se reparten entre los núcleos; la difuminación vertical se hace trasponiendo.
    - **La calidad completa se arma fuera de la pantalla.** Al quedarse quieta, la ventana seguía mostrando cómo se dibujaban las franjas de arriba abajo, y al final los retoques (viñeta, resplandor) aparecían de golpe. Ahora las franjas se renderizan en un búfer aparte (se sigue atendiendo el teclado entre una y otra, y moverse la cancela), y cuando está entera, con los retoques, reemplaza a la vista previa de una sola vez. Al frenar, el único cambio es que la imagen se enfoca.
    - **Resultado:** ~15–25% más rápido según la vista. Lo que queda es sobre todo el costo de las velas y de las lámparas colgantes (que llegan a 12 m), y los rayos primarios. Bajarlo más ya cambia algo de la imagen (ver las opciones que se le propusieron al autor).

23. **Los padres de los novios, comidos a lo largo de la nave ✅**
    - **Dónde:** en el camino de entrada hacia la boda, no junto a la puerta ni pegados a la escultura de los novios. Cuatro de los zombis que caminaban por la nave pasaron a ser los atacantes, en los mismos lugares (de la puerta hacia el altar):

      | Lugar | Escena (`zombies::feast`) |
      |---|---|
      | ~13 m, lado sur | **El padre del novio**, de pie, peleando, de cara a la puerta. Los zombis lo atacan por los costados para que se le vea de frente al entrar: uno colgado de su brazo estirado, mordiéndoselo, y él empujándole la cabeza al otro |
      | ~17 m, lado norte | **La madre de la novia**, arrastrándose boca abajo hacia el pasillo central, de perfil para quien entra, con la cara levantada gritando. Un zombi echado sobre su espalda, corrido hacia el lado de atrás, le muerde el hombro de ese lado |
      | ~21 m, lado sur | **La madre del novio**, desplomada contra un reclinatorio, mirando hacia la entrada, con la cabeza torcida. Un zombi arrodillado le muerde el cuello y su sombrero quedó tirado al lado |
      | ~23.5 m, lado norte | **El padre de la novia**, tirado de espaldas, con dos zombis arrodillados comiendo, uno del cuello y otro del brazo; él empuja la cara de uno con la mano |

      Cada escena tiene sus heridas (sangre que cae hacia el piso), salpicaduras debajo y un charco.
    - **Moda de boda de los 80** (`people::Outfit`, materiales nuevos):
      - padre de la novia: esmoquin gris perla, camisa con volados, moño y faja bordó, canoso, bigote;
      - padre del novio: traje azul marino, corbata ancha fucsia, castaño, bigote;
      - madre de la novia: raso fucsia con mangas abullonadas enormes, perlas, permanente castaña;
      - madre del novio: verde azulado con hombreras, permanente rubia, sombrero haciendo juego con cinta fucsia.

      Todos con la cabeza cuadrada del resto del cortejo y la boca abierta en un grito.
    - **Código:**
      - el esqueleto de cápsulas pasó a `scene/figure.rs`, compartido por zombis y personas (pose, articulaciones, partes del cuerpo, ajuste del canvas). Las partes ahora distinguen torso, brazo, antebrazo, mano, muslo y pierna, para vestirlas;
      - `people::dressed` viste una pose;
      - `zombies::feeding` es un zombi comiendo, arrodillado o parado, inclinado lo justo para llegar con la boca a la mordida.
    - **Costo:** cuatro grids chicos más; la vista previa sigue en 0.1–0.3 s.

24. **La cámara orbital no puede salir del edificio ✅**
    - **El problema:** con la puerta principal y el vitral del ábside rotos, alejar la cámara orbital (zoom) o girarla (orbit) podía sacarla del edificio, hacia un vacío que no tiene nada modelado (el proyecto es solo interior). Un primer arreglo usó una sola caja envolvente (`VoxelGrid::bounds()`, el rectángulo que contiene a todo el edificio) para frenar la cámara — alcanzaba para el zoom en línea recta, pero no para girar: la planta es en cruz (la nave es angosta, el crucero es ancho), así que esa caja única deja afuera, pero "adentro de sus límites", las cuatro esquinas entre el ancho de la nave y el del crucero — aire vacío de afuera, nunca tallado. Girar la vista "nave" hacia el costado mandaba la cámara justo ahí.
    - **La solución** (`Camera::bounds`, ahora `Vec<(Vec3, Vec3)>` en vez de una sola caja): el volumen seguro es la unión de una caja por cada sala real (`cathedral::camera_rooms`): nártex + vestíbulos, nave + naves laterales, crucero, coro + ábside. Cuando `orbit()` o `zoom()` dejarían a `eye` fuera de todas las cajas, se lo manda al punto más cercano de la caja más próxima — como deslizar contra la pared más cercana en vez de atravesarla.
    - `Scene::camera_bounds` es `Vec::new()` (sin límite) para la iglesia original (`--church`), que sí tiene vistas exteriores a propósito (su grid incluye la plaza alrededor) — ponerle el mismo límite habría arruinado esas tomas, hechas con la cámara bien afuera del edificio a propósito.
    - **Tests:** además de los de `camera.rs` (con una forma en cruz sintética), uno de integración en `cathedral.rs` (`orbital_camera_cannot_leave_the_cathedral`) corre las 12 vistas reales, alejando y girando la cámara al límite, y compara contra `camera_rooms()` calculada aparte (no contra lo que de hecho usa la cámara) — así detecta si `build()` alguna vez volviera a usar una sola caja. Confirmado a mano: con una sola caja, este test falla a los pocos pasos de girar la vista "nave" (la cámara cae en el hueco junto a la nave); con las cuatro cajas, pasa. Otro test (`preset_eyes_are_not_moved_by_their_own_bounds`) confirma que ninguna de las 12 vistas se corre de su encuadre original al aplicar el límite.

25. **Placa de título y escena de introducción ✅**
    - Antes del diorama, dos imágenes a pantalla completa, una tras otra, cada una hasta que se aprieta cualquier tecla (`intro.rs`): una "placa de título" y una "escena de introducción", generadas aparte (con Manus, a partir de referencias del autor). Es un agregado opcional, igual que el modo primera persona.
    - **Dónde van:** `assets/intro/title.png` y `assets/intro/scene.png` (ya puestas por el autor). Si faltara alguna, se saltea sola — se avisa por consola y el programa sigue directo al diorama, sin romper nada.
    - **Formato:** PNG (el único decodificador habilitado en el proyecto). Cualquier tamaño sirve — se reescala a la ventana (vecino más cercano, `intro::load_scaled`) —, pero para que no se vea estirado conviene generarlas en 16:9, idealmente 1280×720 (el tamaño de la ventana en calidad completa).
    - **El cartel** ("PRESS ANY KEY TO CONTINUE", pedido por el autor) se dibuja en código, no viene en las imágenes: una tipografía de píxeles mínima hecha a mano (`font.rs`, 5×7, las letras que hacen falta para ese mensaje), sobre una franja oscurecida al pie de cada imagen para que se lea encima de cualquier fondo.
    - `--no-intro` la saltea sin más (para iterar rápido durante el desarrollo); solo aplica a la ventana interactiva, no a `--snapshot`.
    - **Tests:** `font.rs` (el espacio no prende píxeles, una letra sí, nada se dibuja más allá del ancho calculado) e `intro.rs` (un PNG de prueba de 2×1 estirado a 4×2 queda bien reescalado y empaquetado; un archivo inexistente no revienta, da `None`).

26. **Los cuatro de la boda, más finos y con mejor silueta ✅**
    - El pedido: no más detalle fino (dedos, arrugas), sino que la silueta general se lea mejor. La resolución sube (2.5 cm en vez de 5, `zombies::SCULPT_DETAIL = DETAIL / 2.0`) solo para este grupo — novios, oficiante y el zombi que muerde —, nada más: el resto de la invasión y los props se quedan en 5 cm, así el costo extra no se nota en ninguna otra vista.
    - **El novio y el oficiante dejaron el sistema viejo** (`skeleton::Body`, de grilla entera y coordenadas discretas) y pasaron al mismo sistema de cápsulas en pose libre que ya usaban los zombis, la novia y los padres (`figure.rs` + `people::dressed`, sobre un `Canvas` propio). `skeleton::Body`/`Facing`/`Role` siguen enteros — los sigue usando `skeleton::build`, los esqueletos con musgo de la iglesia original (`--church`).
    - Esto significa que `Canvas::new` ahora recibe el tamaño de celda como parámetro (antes era siempre la constante `DETAIL`); los seis lugares que ya lo llamaban se actualizaron para seguir en 5 cm, y solo la boda pasa `SCULPT_DETAIL`.
    - **`Outfit` y `dressed()` (`people.rs`) se ampliaron** para vestir también al novio y al oficiante, no solo a los padres:
      - `Neckwear` (un enum: nada, perlas, moño o corbata) reemplaza al booleano `necklace` de antes — hacía falta un cuarto caso ("nada") para el alba, que no lleva nada al cuello;
      - `Skirt` (material + hasta dónde llega + los dos radios) reemplaza a `skirt: Option<Material>` — antes la falda medía siempre lo mismo (hasta la rodilla), y el alba necesita llegar casi al piso;
      - `stole: Option<Material>`, nuevo: dos tiras verticales desde el cuello (la estola del oficiante no es una faja a la cintura, es otra prenda).
    - **Las poses, de pie y con los gestos que ya tenían:** el novio con un pie atrás y las dos manos a los costados de la cabeza; el oficiante con las dos manos bien arriba y el libro caído delante (`people::fallen_book`, como `fallen_hat`).
    - **Un bug de diagnóstico, no de la escena:** al revisar de cerca, el oficiante parecía un poste liso sin brazos ni cabeza. La geometría estaba bien — era la cámara de prueba, mirando casi exactamente a lo largo de su propio eje de hombros, que aplastaba los brazos levantados contra el torso en la proyección. Desde cualquier ángulo normal (las vistas "boda", "mordida", "altar") se lo ve de pie, con los brazos claramente separados.
    - **Costo:** medido, no estimado. La vista previa no cambia (0.1–0.2 s en todas las vistas). En calidad completa, las vistas que miran de cerca al grupo — "boda", "mordida", "altar" — pasan de ~1–1.8 s a 1.7–2.5 s (más de lo que se había previsto como "unos pocos por ciento", porque en estas vistas de cerca la mayoría de los rayos del cuadro SÍ atraviesan el grupo, no es un costo tan acotado como en una vista lejana). El resto de las vistas no cambia.

27. **La boda esculpida: personas, no muñecos ✅**
    - **El pedido:** que los cuatro de la boda parezcan personas de verdad y no dibujos animados, con proporciones anatómicas reales, "esculpidos y trabajados". Resolución: se queda en 2.5 cm (decisión del autor). Lo que se veía mal, según el autor: no se le veía la cara a la novia; la falda y las piernas no se comportaban como tela; el zombi, el novio y el oficiante no parecían personas. Lo único que ya estaba bien: el velo y la sangre (no se tocaron).
    - **Por qué se veían como muñecos:** cada cuerpo eran ~12 cápsulas (caños del mismo grosor de punta a punta, un cono de torso, una bola de cabeza), sin caja torácica, cadera, hombros ni pies, de un solo color plano; y la falda era un cono rígido que seguía el torso inclinado, con un caño de pierna asomando.
    - **Escultura por campos de distancia** (`sculpt.rs`, nuevo): cada figura es "arcilla" — elipsoides, conos redondeados y bloques que se funden entre sí con una unión suave (`smin`), y huecos tallados con una resta suave (cuencas, boca). Se arma en capas de adentro hacia afuera (`Layer`: la piel, la camisa, el saco...), cada una pintando encima de la anterior; la pintura puede dejar celdas sin pintar (el saco abierto deja ver la camisa). `Canvas::fill` es la base.
    - **Tela que cae por gravedad** (`Drape`): una capa con caída cuelga de lo que la sostiene (la cadera, los muslos, la rodilla levantada) hasta el piso, abriéndose a medida que baja, con pliegues verticales que se hacen más hondos cuanto más cae, y juntándose sobre el piso. La falda de la novia ahora cae recta de la cadera aunque el torso esté a 70°, se apoya sobre la rodilla levantada y termina en una cola en abanico; el alba del oficiante cae en pliegues y sus mangas anchas cuelgan del brazo.
    - **Anatomía** (`anatomy.rs`, nuevo): un esqueleto con proporciones reales (7.5 cabezas; pelvis a media altura; brazo 0.30 + antebrazo 0.255 m, muslo 0.45 + pierna 0.43 m para 1.80 m, escalado por la altura de cada uno) y encima las masas: pelvis y glúteos, abdomen, caja torácica, pectorales o busto, trapecios y clavículas, deltoides, bíceps, antebrazos que se afinan, muslos, rodillas, pantorrillas, pies con talón, manos con palma, dedos juntos y pulgar, y una cabeza con cráneo, cara, mandíbula, mentón, pómulos, cejas, nariz, orejas, cuencas y la boca abierta en un grito. Tres complexiones: hombre, mujer y cadáver. `reach_arm`/`reach_leg` resuelven el codo o la rodilla para llevar la mano o el pie a un punto (las manos del novio a la cabeza, los pies del zombi al piso).
    - **Cada figura:**
      - *La novia* (1.68 m): corpiño strapless de raso con escote corazón, cinta a la cintura, falda con caída y cola, pelo recogido con rodete y mechones sueltos que cuelgan, labios pintados. La cabeza echada hacia atrás y **girada hacia la nave** (antes miraba al techo, tapada por el zombi): el zombi le muerde el costado del cuello que le queda al aire, del lado de atrás, así su cabeza no le tapa la cara. Con una mano le agarra la muñeca al zombi (antes iba a su hombro y el brazo le cruzaba la cara).
      - *El novio* (1.80 m): frac — saco de lana negra con solapas de raso, abierto sobre la pechera blanca, faja negra, colas hasta las rodillas, moño, boutonnière roja, pantalón que cae recto, zapatos de charol. Las manos a los costados de la cabeza con los codos hacia adelante; el cuerpo y la cara girados hacia la nave (pedido del autor).
      - *El oficiante* (1.78 m): alba de lino crudo con caída, mangas anchas, estola bordó que baja por el pecho y cuelga recta, canoso y pelado arriba.
      - *El zombi*: el mismo zombi desnudo y amarillento de la invasión (pedido del autor), pero esculpido y más podrido — costillas, esternón, columna y omóplatos marcados, el vientre hundido y abierto con las tripas colgando, miembros flacos con articulaciones nudosas, cuencas vacías, sin nariz, mejillas hundidas, la mandíbula abierta con dientes (con huecos), parches de carne oscura y llagas. Encorvado sobre ella desde atrás, con las rodillas dobladas y los pies apoyados afuera de la falda.
    - **Dos agregados al `VoxelGrid`**, solo para estas figuras: *oclusión horneada* (`bake_occlusion`: los pliegues, axilas, cuencas y comisuras se oscurecen) y, sobre todo, *normales suavizadas* (`smooth_normals`). Con la luz de frente, una superficie inclinada de vóxeles se veía rayada — cada escalón con la cara de arriba iluminada y la de adelante oscura —, y eso era lo que hacía que la falda pareciera tejida al crochet. Ahora cada celda de superficie guarda la dirección contraria a la masa que la rodea y la luz sigue la forma esculpida; la silueta sigue siendo de vóxeles. El velo (transparente) no se toca.
    - **Materiales nuevos:** raso liso para el vestido (el de antes tenía textura y a esta escala parecía tejido), lana negra y raso negro para el frac, labios, rouge, el fondo de la boca, iris, y carne podrida oscura.
    - **Bug encontrado en el camino:** la unión suave arrancaba desde infinito y `∞ × 0` daba NaN, así que toda capa con unión suave quedaba vacía (solo se veían las colas del frac, la cola del vestido y la sangre). Arreglado y con test.
    - Los padres de los novios siguen con el sistema anterior (`figure::realistic_body` + `dressed`) a 5 cm: no eran parte del pedido.
    - **Tests:** las formas son negativas adentro y positivas afuera; la unión suave llena la muesca entre dos formas y arranca bien desde infinito; la tela cuelga hacia abajo hasta el piso y no sube ni lo atraviesa; la novia mira hacia la nave, la boca del zombi está en la mordida y su cabeza queda detrás de la de ella; los pies del zombi apoyan en el piso afuera de la falda.
    - **Costo:** las vistas de cerca incluso bajaron un poco ("boda" 3.0 → 2.3 s, "mordida" 2.5 → 2.3 s, "altar" 2.3 → 1.7 s; las formas esculpidas son más compactas que las cápsulas de antes). Armar la escena tarda ~2 s más al arrancar (esculpir por campos de distancia, hornear la oclusión y las normales).

28. **Ajustes de la boda: el novio de frente, el zombi de pie ✅**
    - **El novio:** al autor le encantó la pose, pero las manos y los brazos le tapaban la cara y costaba leer el gesto. Ahora las manos van sobre la cabeza (detrás de la cara) con los codos abiertos a los costados, así los brazos enmarcan la cara; el cuerpo y la cara giran más hacia la nave. Se corrió un paso a la izquierda (1.15 m al costado de la novia) para no chocar con el zombi.
    - **El zombi:** ya no está agachado. Está de pie y erguido (pelvis a 0.81 m, torso a 36° de la vertical), parado detrás del cuello de la novia, de frente a la nave, sosteniéndola como en el paso de baile de una boda (un brazo bajo la espalda, otro en la cintura), con la cabeza agachada mordiéndola. Desde la nave se le ven la cabeza y los hombros por encima de la cabeza de ella y las piernas por debajo de su cuerpo, sin taparle la cara.
    - **Por qué así y no de perfil:** se probaron dos posiciones de perfil (del lado de los pies de ella). Con la novia muy inclinada, para llegar al cuello de pie, el torso del zombi quedaba casi horizontal, acostado sobre ella; con la novia más erguida, los dos torsos quedaban paralelos y el de él, justo detrás del de ella. La única forma de verlo entero desde la nave es que esté del otro lado de ella, erguido.
    - **La novia**, para que alguien de pie la alcance: la cadera un poco adelantada y el torso a 50° de la vertical (antes 70°), como en el paso de baile real. La cara sigue girada hacia la nave.
    - `dip_zombie` (separada de `dip`) prueba cada inclinación del torso, con la boca puesta en la mordida, y se queda con la que deja la pelvis a la altura de alguien parado. Los pies se acercan a la cadera si quedaron fuera de alcance de la pierna. La falda y la cola se abren donde las empujan las piernas del zombi (`dipped_bride(.., pressed)`): `Layer::carve` ahora se resta donde termina la tela, no donde cuelga.
    - **Tests:** el zombi tiene los pies apoyados, está de pie y erguido, detrás de ella, del lado opuesto a la nave, y con el cuello por encima de la cabeza de ella; una pierna que empuja una falda con caída la abre ahí y no del otro lado.

29. **Carteles y vista inicial ✅**
    - **Intro:** la placa de título sigue con "PRESS ANY KEY TO CONTINUE"; la escena (los novios antes del ataque) ahora dice "SAY CHEESE! PRESS ANY KEY TO TAKE THEIR HAPPY DAY PICTURE".
    - **Vista inicial** (`inicio`, la primera de la lista): el vitral reventado del ábside con los zombis entrando y la luna detrás, desde pasando el altar (que antes tapaba el cuadro) y delante de la cascada. (Tuvo un cartel "OH NO! PRESS S TO SEE WHAT HAPPENED", sacado en la fase 30.) S aleja la cámara: retrocede por encima del altar y de la boda, que queda a la vista con el vitral detrás.
    - **La cámara no pasa por detrás de los zombis:** alejándose (S) desde la vista "nave", antes retrocedía hasta el nártex, detrás de la montaña de la puerta. Medido sobre los grids de la invasión, la montaña llega hasta y ≈ 30 y la cascada del ábside empieza en y ≈ 196. `camera_rooms` ya no tiene caja para el nártex: la nave arranca en `DOOR_HEAP_FRONT` (31) y el ábside termina en `APSE_TORRENT_FRONT` (196). Test: desde cualquier vista, alejándose o acercándose al máximo, la cámara queda entre las dos.
    - La tipografía de píxeles sumó H, D, W y "!".

30. **El novio más flaco y con piernas; sin cartel en el diorama ✅**
    - **El novio** se veía gordo y como si le faltara media pierna. Tres causas: el saco engrosaba un torso que ya era ancho; el saco rodeaba toda la cadera; y las colas ensanchaban la silueta hasta las rodillas, así que de frente las piernas parecían arrancar a mitad de muslo. Además, las dos piernas del pantalón se fundían en una sola columna hasta la rodilla. Ahora: cintura y espalda masculinas más finas (`anatomy.rs`; también afinan al oficiante), saco más delgado que solo cubre el asiento por detrás, colas más angostas, pantalón más fino y la postura un poco más abierta, así se ven dos piernas desde la entrepierna.
    - **El cartel "OH NO! PRESS S..."** se sacó a pedido del autor. La vista inicial sigue siendo el vitral del ábside.
    - **"La vista por defecto no tiene que dejar ver a los zombis"** se refería solo a la montaña de la puerta (el resto de los zombis está bien). Desde la vista inicial la cámara siempre mira hacia el vitral, y las salas de la cámara terminan antes de la cascada: la puerta queda siempre a la espalda. Test: alejándose al máximo y girando una vuelta entera desde "inicio", la montaña de la puerta nunca queda adelante de la cámara.

31. **Desde el vitral hasta la entrada con S; el novio más alto ✅**
    - **El pedido:** desde la vista inicial (el vitral del ábside), apretando S hay que poder retroceder hasta la entrada y ver la iglesia entera, sin pasar por detrás de la montaña de zombis de la puerta.
    - **Por qué no llegaba:** el alejamiento tenía un tope de 4 veces la distancia inicial (`max_radius`), que lo frenaba a la altura del crucero; y la vista inicial miraba un poco hacia arriba, así que al retroceder la cámara bajaba hasta el piso.
    - **Ahora:** con un volumen seguro (`Camera::with_bounds`) ya no hay tope de radio, el límite son las paredes; y la vista inicial está a 4 m de altura mirando derecho al vitral, así que S la lleva hacia atrás a esa altura, por encima del altar y de la boda, hasta la entrada (se frena delante de la montaña de la puerta, en `DOOR_HEAP_FRONT`). Test: alejándose desde "inicio" se llega a menos de 6 unidades de la montaña, sin pasarla y por encima del piso.
    - **El novio:** de 1.80 a 1.88 m, y cadera más angosta (la pelvis y los glúteos masculinos, el pantalón y el saco sobre la cadera, más finos).

32. **Una pantalla negra ominosa entre la foto y el diorama ✅**
    - Después de "SAY CHEESE!...", antes del diorama (que arranca en el vitral reventado): una pantalla negra con "...WAIT. DID YOU HEAR THAT?" (elegido por el autor entre cuatro opciones), en gris pálido (del mismo tamaño que el cartel de la foto, desde la fase 37).
    - El texto aparece desde el negro en 2 s y el diorama arranca solo a los 4 s; cualquier tecla lo adelanta. Sin cartel de "apretá una tecla" (pedido del autor: a esta altura ya se sabe).
    - Es solo código (`intro::omen`, `omen_frame`): no necesita imagen ni toca el render. La tipografía sumó "." y "?". `--no-intro` la saltea junto con el resto de la intro.
    - **Test:** arranca en negro, a mitad del fundido el texto es más tenue y al final queda visible y centrado.

33. **El novio sin colas ✅** — Las colas del frac colgaban por detrás hasta las rodillas y, de frente, se veían por el hueco entre las piernas como algo raro; al autor no le gustaban. Se sacaron: el saco queda cortado a la cintura y entre las piernas se ve el piso (y las piernas se leen más largas).

34. **El novio, hundido en el piso; los invitados ordenados ✅**
    - **El novio estaba 30 cm metido en el piso.** Al correrlo un paso (fase 28) el desplazamiento se pasó como altura en `meters(x, y, z)` (la tercera coordenada es la altura, no el largo). Eso explicaba que pareciera más bajo y "sin media pierna". Ahora está apoyado en el piso, un paso hacia la entrada, y mide lo mismo que el oficiante (1.80 m contra 1.78 m).
    - **Los padres, revisados sin reconstruirlos** (pedido del autor: misma escala y mismo sistema de cápsulas a 5 cm):
      - **Ubicación pareja:** a 2 m del eje, uno de cada lado del pasillo central por turno, cada 3.6–4 m (antes zigzagueaban irregulares, uno casi en el pasillo).
      - **Los zombis que comen vienen siempre del lado de atrás de la víctima** (`far_side`: el que no da a la entrada), así desde la nave se ve a cada víctima entera y al zombi de frente mordiendo. La madre que se arrastra ya no tiene un zombi acostado encima: uno arrodillado a su lado le muerde el hombro de atrás. El padre tirado de espaldas tiene un solo zombi (al cuello) en vez de dos que lo tapaban.
      - **La permanente de las madres**, de tamaño creíble (se leía como bultos).
      - **El zombi que se arrastra con su rastro de sangre** pasó al otro lado del pasillo: quedaba tirado delante de la madre desplomada, tapándola.

35. **Los dos padres que se ven primero, esculpidos ✅**
    - El padre del novio y la madre de la novia (los dos más cerca de la entrada) se rehicieron con el mismo sistema que la boda (`anatomy.rs` + `sculpt.rs`, a 2.5 cm); los otros dos padres siguen con el esqueleto de cápsulas a 5 cm (decisión del autor: la mitad del trabajo, donde más se ve).
    - **El padre del novio** (`people::suited_father`, 1.78 m): traje azul marino de los 80 con hombreras, saco cerrado con la V angosta sobre la camisa blanca y la corbata fucsia, pantalón recto, zapatos negros, pelo corto y bigote, gritando. De pie, de cara a la entrada: el brazo de un costado estirado con un zombi de pie mordiéndole el antebrazo (agarrándole el brazo con las dos manos); con la otra mano le empuja la cara a otro que se le tira encima desde el otro costado, con la cabeza echada para atrás.
    - **La madre de la novia** (`people::crawling_mother`, 1.65 m): vestido de raso fucsia con mangas abullonadas enormes, la falda hasta la rodilla cayendo sobre el piso alrededor de las piernas (la misma tela con caída de la boda), collar de perlas, permanente castaña, zapatos negros y labios pintados. Boca abajo, apoyada en un codo, con el otro brazo estirado hacia el pasillo; un zombi arrodillado del lado de atrás le muerde el hombro y la agarra de la espalda y del brazo.
    - **Los zombis** son el mismo cadáver esculpido y podrido del de la mordida (`sculpted_zombie`). `biting_zombie` arma uno con la boca en la mordida, de pie o arrodillado (prueba cada inclinación del torso hasta que la pelvis quede a la altura justa); `grab` lleva sus manos a lo que agarran.
    - **Pies estirados** (`anatomy.rs`): si el pie apunta en la línea de la pierna, queda con el empeine apoyado (arrodillado, o boca abajo en el piso), en vez de en ángulo recto con la pantorrilla.
    - **Costo:** ninguno medible. Armar la escena sigue en ~1.5 s y las vistas tardan lo mismo.

36. **Los otros dos padres, esculpidos; fuera el sistema viejo ✅**
    - **La madre del novio** (1.68 m, `Dress::groom_mother`): vestido verde azulado de los 80 con hombreras y mangas largas, perlas, permanente rubia, zapatos negros. Sentada en el piso, desplomada contra el reclinatorio, una pierna estirada y la otra con la rodilla levantada (la falda cae sobre la rodilla y se junta en el piso); la cabeza caída hacia la entrada deja el cuello al aire del lado de atrás, donde muerde un zombi arrodillado, al que empuja débil del pecho. El sombrero, tirado delante.
    - **El padre de la novia** (1.75 m, `Suit::bride_father`): esmoquin gris perla abierto hasta la cintura, pechera con volados, moño y faja bordó, pelo canoso y bigote. Tirado de espaldas con una rodilla levantada, grita hacia la entrada; un zombi arrodillado le come el cuello del lado de atrás y él lo empuja de la cara; el otro brazo, tirado en el piso hacia quien llega.
    - `suited_father` y `eighties_mother` quedaron generales (`Suit`: tela, corbata o moño, esmoquin o saco con hombreras, pelo; `Dress`: raso, pelo, mangas abullonadas o largas con hombreras), así los cuatro padres se visten con lo mismo.
    - **Fuera el sistema viejo de personas:** `Outfit`, `dressed`, `Neckwear`, `Skirt`, `figure::realistic_body`/`realistic_head`/`hand_shape`, `zombies::feeding` y el material `cloth` ya no los usaba nadie. `figure.rs` queda solo para los zombis de la invasión.
    - **Costo:** armar la escena tarda ~1.5–2.8 s (antes ~1.5 s); las vistas, lo mismo (medido tres veces: las mediciones sueltas varían mucho con la carga de la máquina).

37. **La escena de la intro como foto, y la gitignore ✅**
    - La imagen de la escena ya no ocupa toda la pantalla: es "la foto", centrada sobre negro con sus proporciones, hasta el 70% del alto (u 80% del ancho), con "SAY CHEESE!..." debajo, fuera de la foto (`intro::photo_size`, `photo_frame`). La placa de título sigue a pantalla completa.
    - Las imágenes que hay que achicar se achican con un filtro suave (la foto pasa de 2560×1440 a 896×504; por vecino más cercano quedaba serruchada); agrandar sigue siendo por vecino más cercano.
    - Test: la foto queda centrada, con negro alrededor, con sus proporciones y el cartel debajo y fuera de ella.
    - Ajuste del autor: la foto, más chica (hasta el 48% del alto o el 56% del ancho: 614×345 en una ventana de 1280×720), y el aviso de la pantalla negra ("...WAIT. DID YOU HEAR THAT?") del mismo tamaño que el cartel de la foto (antes era más grande).
    - `.gitignore` suma `skills/` y `*.skill` (las skills de Claude no van al repo; `.claude/` ya estaba ignorada).

38. **El aviso de la pantalla negra, en relación con la foto ✅** — Después de "SAY CHEESE! PRESS ANY KEY TO TAKE THEIR HAPPY DAY PICTURE", el aviso ahora sigue a la foto recién sacada: "*CLICK* ...WAIT. THERE'S SOMETHING ELSE IN THE PICTURE." (elegido por el autor entre cuatro; el anterior, "...WAIT. DID YOU HEAR THAT?", no tenía que ver con la foto). La tipografía sumó L, M, G, el apóstrofo y el asterisco. De paso: `font::text_width` contaba una columna de espacio de más después de la última letra, así que todos los carteles centrados quedaban corridos medio glyph a la izquierda.

39. **El aviso, más auténtico ✅** — El autor pidió algo menos "de miedo" y más natural, lo que pensaría de verdad el invitado que sacó la foto: "*CLICK* ...THAT CAME OUT WEIRD. SOMETHING'S IN THE BACK." (elegido entre diez). La tipografía sumó la B.

40. **El flash de la cámara en vez de la pantalla con texto ✅** — El autor sentía el paso foto → pantalla negra con texto → diorama poco natural. Lo más fuerte de la intro es el antes y el después (la foto tranquila, después la masacre); lo que molestaba era la pantalla del medio, que contaba en vez de mostrar. Ahora, al apretar una tecla sobre la foto, salta el flash de la cámara: blanco pleno 90 ms que se apaga hasta negro en 650 ms (rápido al principio, lento al final), sin texto — el flash es el "click" —, y arranca el diorama igual que antes (en el vitral del ábside, pedido del autor). `intro::flash`, `flash_level`; test: arranca en blanco, se apaga sin volver a subir y termina en negro.

41. **El zombi de la mordida, más visible ✅** — Sin cambiarle la pose: la cabeza 20% más grande y los brazos (y los bíceps) 40% más gruesos, con las manos un poco más grandes. `Skeleton` sumó `head_scale` y `arm_scale` (1 = proporción real, lo de todas las demás figuras); la boca se sigue poniendo en la mordida después de agrandar la cabeza, así que sigue mordiendo el cuello de la novia.

---

## 5. Especificación original de Manus

Se transcribe tal como la entregó Manus. La hoja de dibujos SVG que menciona no está
disponible; las coordenadas y tablas de abajo son la fuente. **La sección 3 manda sobre
esta.**

# Sainte-Cécile — Procedural Voxel Cathedral Design Specification

**Purpose:** a deterministic, symmetric Gothic/Victorian European cathedral blockout suitable for a Rust CPU raytracer. This is a build specification, not a freeform concept-art prompt. The attached drawing sheet is schematic; the coordinates and schedules below are authoritative.

## 0. Grid, orientation, and construction rules

- **Base voxel:** 0.20 m = 1 voxel. Every architectural boundary, size, and placement snaps to this grid. Convert with `voxel = round(metres / 0.20)`.
- **Detail-prop voxel:** 0.05 m = 1 prop voxel, only for the explicitly listed props. Convert with `prop_voxel = round(metres / 0.05)`. Props are separate meshes/voxel volumes; do not apply 5 cm resolution to the cathedral shell.
- **World origin:** `(x=0, y=0, z=0)` is the cathedral centreline at the west façade and finished interior floor. `x<0` is north, `x>0` south; `y=0` is west and positive `y` points east; `z=0` is interior finished floor. Positions below are the **minimum corner** of an axis-aligned volume unless called out as a centre or polygon vertex. Negative coordinates are intentional.
- **Site apron:** ground/plaza extends 6 m (30 voxels) beyond the building bounding footprint on each side. Use a simple dark stone paving plane, not part of the church footprint.
- **Envelope:** cathedral maximum bounding box is 28 m wide × 68 m long × 38 m high = **140 × 340 × 190 base voxels**. The surrounding 6 m apron produces a 40 m × 80 m site rectangle.
- **Style:** pale and charcoal marble, robust stepped pointed arches, deep shadow openings, heavy buttresses, block-built octagonal spires, and repeating ribs. Avoid sub-voxel tracery, slender pinnacles, railings, chains, and decorative strands. Any building-scale raised/recessed trim is at least 0.40 m wide/deep; structural shells are 0.80 m or thicker.
- **Lighting premise:** night. Only the scheduled candle groups and moonlight are emitters. Window apertures not expressly identified as glass remain open dark voids.

## 1. Floor plan — dimensions and coordinates

Use the SVG for the labeled 4 m grid. The footprint is symmetric about `x=0`; grid intersections are 4 m = 20 voxels. The nave's clear span is 12 m, with 4 m aisles each side. Six 8 m bays run from `y=8` to `y=56`.

### Plan zones

| Zone | Plan geometry in metres | Base-voxel geometry | Notes |
|---|---|---|---|
| West façade / narthex block | `x=-11..+11`, `y=0..8`; 22 × 8 m | `x=-55..+55`, `y=0..40`; 110 × 40 vox | Two tower footprints flank a central entry/narthex. |
| North and south towers | each 7.2 × 8 m; north `x=-10.8..-3.6`, south `x=+3.6..+10.8`, `y=0..8` | each 36 × 40 vox | Mirror pair; tower mass rises to 29 m, stepped spire to 38 m. |
| Nave + side aisles | `x=-10..+10`, `y=8..56`; 20 × 48 m | 100 × 240 vox | Six repeated bays, each 8 × 20 m. |
| Central nave | pier centre-lines `x=-6..+6`, `y=8..56`; 12 × 48 m nominal structural span | 60 × 240 vox | Clear between 1.6 m pier faces is 10.4 m (52 vox); aisle widths are 4 m between structural grid lines. |
| Side aisles | north `x=-10..-6`, south `x=+6..+10`, `y=8..56`; each 4 × 48 m nominal grid span | each 20 × 240 vox | Aisle vault crown 11 m; clear walking width beside the 1.6 m arcade piers is about 2.4 m (12 vox). |
| Transept | `x=-14..+14`, `y=40..48`; 28 × 8 m | 140 × 40 vox | Crosses nave bay 5; arms project 4 m beyond nave walls. |
| Choir | `x=-6..+6`, `y=56..60`; 12 × 4 m | 60 × 20 vox | Leads to the apse; high altar centred east. |
| Octagonal apse | vertices: `(-6,60),(6,60),(6,62),(4,66),(2,68),(-2,68),(-4,66),(-6,62)` m | vertices ×5: `(-30,300),(30,300),(30,310),(20,330),(10,340),(-10,340),(-20,330),(-30,310)` vox | East end has a 4 m-wide flat face, sufficient for the one stained-glass panel. |
| Main stained-glass opening | centred on east face `x=0, y=68`; 3.2 m wide × 5.2 m high; sill `z=4.8 m` | 16 × 26 vox; sill `z=24 vox` | One pointed panel only. Lead divisions are chunky 0.4 m (2-voxel) bands. |

**Plan extents:** west façade `y=0`; eastmost apse face `y=68`; max transept width `x=±14`; plaza perimeter `x=±20`, `y=-6..74`. Floor elevations are `z=0` finished floor; stone subfloor drops to `z=-0.4` (2 voxels).

## 2. Repeated bay module

Repeat **one 8.0 m-long × 20.0 m-wide bay six times** along the nave. Bay `i=0..5` occupies `y=8+8i .. 16+8i`; repeat origin is `y=8+8i` (40 voxels per step). The module drawing shows its plan and nave arch profile.

### Module dimensions and construction

| Module component | Size (m) | Size (base vox) | Placement within bay (m) | Placement from bay origin (vox) |
|---|---:|---:|---|---|
| Full nave + aisle bay | 8.0 longitudinal × 20.0 transverse | 40 × 100 | `x=-10..+10`, `y=0..8` | `x=-50..+50`, `y=0..40` |
| Nave support-grid span | 12.0 × 8.0 centreline-to-centreline | 60 × 40 | `x=-6..+6`; 10.4 m (52 vox) remains clear between 1.6 m piers | `x=-30..+30` |
| Each aisle | 4.0 × 8.0 | 20 × 40 | `x=-10..-6`, `+6..+10` | `x=-50..-30`, `+30..+50` |
| Inner compound pier | 1.6 × 1.6 footprint; 16.0 high to springing | 8 × 8 × 80 | centres at `x=±6`, `y=0` and `y=8` | centres at `x=±30`, `y=0/40` |
| Outer wall pier | 0.8 × 1.6 footprint; 16.0 high | 4 × 8 × 80 | centres at `x=±9.6`, `y=0` and `y=8`; flush in the 0.8 m outer wall | centres at `x=±48`, `y=0/40` |
| Longitudinal nave arcade arch | 8.0 m bay span along `y` × 15.2 m apex above floor | 40 × 76 vox | pointed opening on each `x=±6 m` arcade line; spring `z=0`, apex `z=15.2 m` | 12 arches total (2 per bay) |
| Transverse nave vault rib | 12.0 m span across `x`; spring `z=16`, crown `z=22` (6 m rise) | 60 span; 80 / 110 vox above floor | one stepped pointed rib at each bay boundary `y=8+8j`, `j=0..6` | 7 ribs total |
| Nave vault | 12.0 span; spring `z=16`, crown `z=22` | 60 span; 80 / 110 vox above floor | pointed ribbed bay; rib thickness 0.8 m | spring `z=80`, crown `z=110` |
| Aisle vaults | each 4.0 span; spring `z=8.8`, crown `z=11` | each 20 span; 44 / 55 vox | shallow pointed/segmented vault to outer wall | spring `z=44`, crown `z=55` |
| Clerestory opening | 2.4 wide along `y` × 3.2 high | 12 × 16 vox | one centred in each bay on each high nave side; sill 16.8 m, apex 20 m | sill 84 vox, apex 100 vox |
| Aisle exterior buttress | 1.6 projection × 1.2 along `y` × 13.6 high | 8 × 6 × 68 vox | north/south outside wall, centred at `y=12+8i` | centred 60+40i vox |

**Repeat support rule:** at bay boundary `y=8+8j`, `j=0..6`, place 1.6 m square inner compound piers at `x=±6` and 0.8 × 1.6 m outer-wall piers at `x=±9.6` (±48 vox) on the same seven stations. All rise from `z=0` to `z=16` (80 vox). Total: 14 inner + 14 flush outer supports = 28. Outer piers exactly fill the outer-wall thickness; exterior buttresses are separate, six per side, at `y=12,20,28,36,44,52` m (12 total).

**Vault recipe:** construct each bay from five named rib runs: two transverse pointed ribs at its ends, one longitudinal ridge rib, and two diagonal ribs from bay corners to a crown key. Use stepped voxel segments, 0.8 m (4-voxel) rib thickness, no thin tracery. Fill the vault shell as 0.4 m (2-voxel) marble slabs between ribs. Side-aisle ribs are 0.6 m (3 voxels) thick. Transverse ribs are shared on bay boundaries; all ribs and supports align to the 8 m (40-voxel) bay grid.

## 3. Elevations and section heights

### West (front) elevation

- Front face at `y=0`; overall façade width 22 m (110 voxels), excluding transept projection behind it.
- Base façade wall rises to `z=19 m` (95 voxels). Central gable apex reaches `z=27 m` (135 voxels).
- Twin tower bases are 7.2 × 8 m (36 × 40 voxels), centred at `x=±7.2 m` (±36 vox); tower wall/parapet top `z=29 m` (145 voxels). Each gets 0.8 m-high / 0.8 m-wide battlement blocks (4 × 4 voxels) at 1.2 m intervals (6 voxels), never narrow crenels.
- Each tower's octagonal stepped spire rises from `z=29` to `z=38 m` (145 to 190 vox; 45 vox high). Use square-to-octagon stepped tiers: 6.4 m (32 vox) base, 4.8 m (24 vox) intermediate at +3 m (+15 vox), 3.2 m (16 vox) at +6 m (+30 vox), 1.2 m (6 vox) cap at +9 m (+45 vox). Keep the spire solid or hollow with ≥0.8 m (4-voxel) shell.
- Central pointed portal: 4 m wide × 8 m high (20 × 40 voxels), opening dark; surround with 0.8 m (4-voxel) stepped marble archivolts. Two side portals: each 2 m wide × 5 m high (10 × 25 voxels).
- One unglazed carved rose recess: 4.8 m diameter (24 vox), centre `x=0,z=14 m` (70 vox); stone ring thickness 0.8 m (4 vox). Radial divisions are broad blockwork only. It is **not glass**.

### North side elevation

- Side elevation is viewed from `x<0`; length is 68 m (340 vox). Nave and aisle side walls are visible from `y=8..56` (40..280 vox).
- Nave external wall/eave line: `z=23.2 m` (116 vox); main roof ridge `z=28 m` (140 vox); inner nave vault spring/crown `z=16/22 m` (80/110 vox).
- Aisle external wall/eave `z=11.6 m` (58 vox); aisle roof ridge at inner edge `z=14.8 m` (74 vox); aisle vault spring/crown `z=8.8/11 m` (44/55 vox).
- Put six open, dark clerestory lancets along the high side wall, one per bay, centred at `y=12,20,28,36,44,52 m` (60,100,140,180,220,260 vox); each 2.4 m (12 vox) wide, 3.2 m (16 vox) high, sill 16.8 m (84 vox), apex 20 m (100 vox). They are openings, not glazed panels.
- Put six buttresses on each long side, one per bay centre, 1.6 m (8 vox) projection × 1.2 m (6 vox) along the wall × 13.6 m (68 vox) high. Their stepped top is at `z=13.6 m` (68 vox).
- Choir walls rise to `z=16 m` (80 vox); apse walls rise to `z=13 m` (65 vox). The one east-facing apse glass panel occupies `x=-1.6..+1.6` (-8..+8 vox), `z=4.8..10.0 m` (24..50 vox).

### Transverse cross-section through nave at `y=28 m`

| Section feature | Height above floor | Height in voxels | Width/span |
|---|---:|---:|---:|
| Aisle wall/eave | 11.6 m | 58 | aisle is 4 m wide each side |
| Aisle vault spring | 8.8 m | 44 | 4 m span each |
| Aisle vault crown | 11.0 m | 55 | shallow pointed profile |
| Aisle roof inner ridge | 14.8 m | 74 | roof slopes outward to eave at 11.6 m |
| Nave arcade spring | 16.0 m | 80 | 12 m (60-voxel) support-grid span; 10.4 m (52-voxel) clear between pier faces |
| Nave vault crown | 22.0 m | 110 | pointed rib apex |
| Nave roof eaves | 23.2 m | 116 | outer edges of nave roof |
| Nave roof ridge | 28.0 m | 140 | outer roof peak; 6 m above vault crown |
| Tower spire peak | 38.0 m | 190 | highest point; tower only |

The nave section is 20 m (100 vox) between outside aisle walls. Side walls are 0.8 m (4 vox) thick. Inner support piers are 1.6 m (8 vox) square; outer wall supports fit within the 0.8 m (4-voxel) wall thickness. Aisles and nave are vaulted volumes, not filled solids; roof shells are stepped marble slabs.

## 4. Element schedule (coordinate-ready)

Positions are minimum corners in metres, then in base voxels. For repeated elements, the coordinate formula is the placement rule; `N` is the total count. Sizes are `X × Y × Z`. The apse is a polygon prism, so its vertex list supersedes an axis-aligned box.

| ID / element | Position / repeat rule (m → base vox) | Size (m → base vox) | Material |
|---|---|---|---|
| E01 Site plaza slab | `(-20,-6,-0.2)` → `(-100,-30,-1)` | `40×80×0.2` → `200×400×1` as an apron ring, subtracting the church footprint | M01 plaza stone |
| E02 West narthex floor | `(-11,0,-0.4)` → `(-55,0,-2)` | `22×8×0.4` → `110×40×2` | M02 pale marble |
| E03 Nave/aisle floor slab | `(-10,8,-0.4)` → `(-50,40,-2)` | `20×48×0.4` → `100×240×2` | M02 pale marble |
| E04 Transept floor wings | north wing `x=-14..-10`, south wing `x=10..14`, `y=40`, `z=-0.4` | each `4×8×0.4` → `20×40×2` | M02 pale marble |
| E05 Choir floor | `(-6,56,-0.4)` → `(-30,280,-2)` | `12×4×0.4` → `60×20×2` | M02 pale marble |
| E06 Apse floor | polygon vertices from plan, `z=-0.4..0` → `z=-2..0` | polygon bounding box `12×8×0.4` → `60×40×2` | M02 pale marble |
| E07 West façade mass | `(-11,0,0)` → `(-55,0,0)` | `22×0.8×19` → `110×4×95` | M02 pale marble; M03 shadow marble in recesses |
| E08 Twin tower shafts | centres `x=±7.2`, `y=0`, `z=0`; each min corner `(cx-3.6,0,0)` → `(5cx-18,0,0)` | outer envelope `7.2×8×29` → `36×40×145`; hollow shaft `5.6×6.4×29` → `28×32×145`, starting at `(cx-2.8,0.8,0)` | M02 pale marble; 0.8 m (4-voxel) shell |
| E09 Twin stepped spires | each centred on tower; tier heights `z=29,32,35,38` m (`145,160,175,190` vox) | bases 6.4/4.8/3.2/1.2 m square → 32/24/16/6 vox square; 9 m total → 45 vox | M03 blue-grey roof stone |
| E10 Nave north/south outer walls | `x=-10/+9.2`, `y=8`, `z=0` | each `0.8×48×12.4` → `4×240×62` | M02 pale marble |
| E11 Nave high clerestory walls | `x=-6.8/+6`, `y=8`, `z=12.4` | each `0.8×48×10.8` → `4×240×54` | M02 pale marble |
| E12 Inner compound supports | at `x=±6` (±30 vox); `y=8+8j`, `j=0..6`; centre coordinates; min corner is centre minus 0.8 m in x/y | each `1.6×1.6×16` → `8×8×80`; N=14 | M02 pale marble; M03 alternating base blocks |
| E12b Outer wall supports | at `x=±9.6` (±48 vox); `y=8+8j`, `j=0..6`; centre coordinates; min corner `x=center−0.4`, `y=center−0.8` | each `0.8×1.6×16` → `4×8×80`; N=14 | M02 pale marble |
| E13 Main longitudinal arcade arches | on each line `x=-6.4/+5.6`, each bay `y=8+8i`, `i=0..5`; pointed, spring0/apex15.2 | each `0.8×8×15.2` → `4×40×76`; N=12 | M02 pale marble; M03 shadow intrados |
| E13b Transverse nave ribs | across `x=-6..+6` at `y=8+8j`, `j=0..6`; spring16/crown22 | each rib envelope `12×0.8×6` → `60×4×30`; N=7 | M02 pale marble; 0.8 m (4-voxel) thickness |
| E14 Nave vault ribs + shell | bay origins `y=8+8i`, `i=0..5`; ribs at bay boundaries and diagonals | per bay `12×8×6` above spring → `60×40×30`; ribs 0.8 m thick (4 vox), shell 0.4 m (2 vox) | M02 pale marble |
| E15 Side aisle vaults | north/south strips `x=-10..-6`, `+6..+10`; bay origins as E14 | each bay `4×8×2.2` → `20×40×11`; N=12 aisle-bays | M02 pale marble |
| E16 Aisle buttresses | north/south; centres `y=12+8i`, `i=0..5`; x projection outward from wall | each `1.6×1.2×13.6` → `8×6×68`; N=12 | M02 pale marble |
| E17 Nave roof | over nave `x=-6..+6`, `y=8..56` (40..280 vox); eaves `z=23.2` (116 vox), ridge `z=28` (140 vox); two sloping planes | bounding envelope `12×48×4.8` → `60×240×24` | M03 blue-grey roof stone |
| E18 Aisle roofs | over each aisle `x=-10..-6` / `+6..+10`, `y=8..56` (40..280 vox); eave `z=11.6` (58 vox), inner ridge `z=14.8` (74 vox) | each envelope `4×48×3.2` → `20×240×16` | M03 blue-grey roof stone |
| E18a Narthex gable roof | central entry block `x=-3.6..+3.6`, `y=0..8` (0..40 vox); eaves `z=19` (95 vox), ridge `z=27` (135 vox) | envelope `7.2×8×8` → `36×40×40` | M03 blue-grey roof stone |
| E18b Transept arm roofs | two gables over `x=-14..-10` and `x=10..14`, `y=40..48` (200..240 vox); eaves `z=14` (70 vox), ridge `z=16.8` (84 vox) | each envelope `4×8×2.8` → `20×40×14` | M03 blue-grey roof stone |
| E18c Choir gable roof | over `x=-6..+6`, `y=56..60` (280..300 vox); eaves `z=16` (80 vox), ridge `z=19.2` (96 vox) | envelope `12×4×3.2` → `60×20×16` | M03 blue-grey roof stone |
| E18d Apse octagonal roof cap | polygon footprint from plan, extrude from `z=13..18` (65..90 vox); stepped pyramid cap | bounding envelope `12×8×5` → `60×40×25` | M03 blue-grey roof stone |
| E19 Clerestory apertures | two per bay, centred at `x=±6.4`, `y=12+8i`; sill16.8, apex20 | opening `0.8 wall × 2.4 along y × 3.2 high` → `4×12×16`; N=12 | Void; M03 dark recess surround |
| E20 Transept outer walls | north/south-facing arm-end walls at `x=-14/+13.2`, `y=40` (200 vox), `z=0`; centre crossing remains open | each `0.8×8×16.8` → `4×40×84`; N=2 | M02 pale marble |
| E21 Transept end arches/windows | one stepped pointed opening in each arm-end wall, centred at `y=44` (220 vox), unglazed dark | each `0.8×4×8` → `4×20×40`; N=2 | Void, M02 surround, M03 deep reveal |
| E22 Choir walls | `x=-6/+5.2`, `y=56` (280 vox), `z=0` | side walls each `0.8×4×16` → `4×20×80` | M02 pale marble |
| E23 Apse polygon walls | extrude apse plan perimeter from `z=0..13` (0..65 vox); shell thickness `0.8` (4 vox) | bounding box `12×8×13` → `60×40×65`; polygon boundary governs | M02 pale marble |
| E24 Apse stained-glass panel | east face `y=67.2..68` (336..340 vox), `x=-1.6..1.6` (−8..+8 vox), `z=4.8..10` (24..50 vox) | `3.2×0.8×5.2` → `16×4×26`; one assembly | M05–M08 transparent/refractive coloured glass; M09 lead |
| E25 West rose recess | west façade centre `(x=0,y=0,z=14)`; round opening carved through wall | Ø4.8 × 0.8 depth → Ø24 × 4 vox; stone ring 0.8 m (4 vox) | M04 carved trim ring; M03 recess; no glass |
| E26 Portal surround + doors | main portal centre x0; y=0; two leaves min corners `(-1.6,0,0)` and `(0,0,0)` → `(-8,0,0)` and `(0,0,0)` vox | opening `4×0.8×8` → `20×4×40`; each door leaf `1.6×0.2×5.6` → `8×1×28` | surround M02 + M04 stepped archivolts; M10 oak; M09 metal bands |
| E27 Side portals | centres `x=±7`, y=0; one leaf per opening at `x=center−0.8` | each opening `2×0.8×5` → `10×4×25`; each leaf `1.6×0.2×4.4` → `8×1×22` | M02 surround, M10 wood door |
| E28 Belfry louver openings | each tower centre `cx=±7.2`; three west-face openings at `x=cx−2.6,cx−0.6,cx+1.4` (voxel formula `5cx−13,5cx−3,5cx+7`), `y=0`, `z=20` (100 vox) | each `1.2×0.8×5` → `6×4×25`; N=6 | Void; M03 interior |
| E29 Battlement blocks | tower parapets, repeated at 1.2 m (6-voxel) interval | each `0.8×0.8×0.8` → `4×4×4`; approx. 10 per tower | M02 pale marble |
| E30 Candle support brackets / hangers | 23 wall fixtures at their matching candle-group centres; 6 central drop posts on nave axis, bay centres, from `z=12..22` (60..110 vox) | each wall plate `0.8×0.4×1.2` → `4×2×6`; each drop post `0.4×0.4×10` → `2×2×50` | M09 reflective iron |

**Implementation note on wall courses:** All listed extents are whole 0.2 m voxels. Similar stepped roofs and pointed arches should be emitted as explicit voxel stair profiles; do not rasterize a sub-voxel sliver.

## 5. Single stained-glass composition

There is **exactly one glazed stained-glass panel**: the east-facing apse lancet. No clerestory, side, transept, rose, tower, or portal opening is glazed. All other fenestration is an open dark cutout with marble reveal.

- Overall panel: 3.2 m wide × 5.2 m high, `16×26` base voxels, sill `z=4.8 m` (24 vox), pointed head apex `z=10.0 m` (50 vox). Opening frame/reveal is 0.8 m thick.
- Composition is one unified lancet (not multiple windows): perimeter ruby border 0.4 m (2 vox), cobalt field, central amber cross/rose motif, and two pale-glass quatrefoil-like block motifs. Use 0.4 m minimum lead strips (2 vox); no hairline lead or tiny tracery.
- Suggested voxel pattern: 16 columns × 26 rows. Rows 0–3 form the pointed crown stepped inward 2 columns every 2 rows; rows 4–21 hold the 16-column body; rows 22–25 step inward symmetrically. Ruby border occupies 2 cells; cobalt fills the inner field. Amber motif: a 2-cell-wide vertical axis from rows 8–19, 8-cell-wide horizontal axis at rows 13–14, and an 8×4 amber centre block. Pale motifs: mirrored 4×4 blocks centred at columns 4 and 12, rows 8 and 18. Use lead strips between each motif, 2 vox wide.
- Render glass with transmission 0.72, IOR 1.50, roughness 0.12; lead is opaque, dark reflective metal. Keep the glass bright enough to read under low moonlight, but do not add a light source inside it; moonlight transmitted through it is still moonlight.

## 6. Material list (13 total; flat base colors)

| ID | Material | Base color | Texture / render note |
|---|---|---|---|
| M01 | Plaza stone | `#777A7D` | Flat grey stone blocks, sparse moss in seams; matte. |
| M02 | Pale cathedral marble / stone | `#D8D3C9` | Warm off-white marble, broad block courses and faint grey veins; matte-to-satin. Main shell, floor, piers, arches, apse. |
| M03 | Blue-grey marble / roof stone | `#454B51` | Blue-grey marble slabs in broad stepped courses, faint charcoal veining and sparse weathering; low gloss. Roofs, deep recesses, spire tiers. |
| M04 | Carved trim marble | `#B7AD9E` | Slightly darker beige stone, repeating chisel bands; satin. Portal archivolts, rose ring, ledges. |
| M05 | Cobalt stained glass | `#1747A0` | Translucent/refractive blue glass, faint pane mottling; transmission 0.72, IOR 1.50. |
| M06 | Ruby stained glass | `#9A263E` | Translucent/refractive red glass, subtle uneven saturation; same glass settings. |
| M07 | Amber stained glass | `#E6A83B` | Translucent/refractive gold glass; same glass settings. |
| M08 | Pale stained glass | `#D9E6D6` | Translucent/refractive opal/pale green-white glass; same glass settings. |
| M09 | Iron / lead metal | `#34383B` | Reflective dark iron; lead cames and door straps, roughness 0.22. |
| M10 | Oak wood | `#60432E` | Dark European oak grain in broad vertical planks; satin, doors/choir furniture. |
| M11 | Candle wax | `#E6D7BA` | Warm ivory wax with faint broad drips; matte. |
| M12 | Candle flame | `#FFB84A` | Flat amber emissive material; use only at wick/flame locations within a candle group. |
| M13 | Bone with moss | `#D7D0B7` | Pale weathered bone base with irregular dark-olive moss patches `#4C5B32`; used only on the reliquary prop. |

No material beyond these 13 is required. Background through openings is the night environment, not a black painted material.

## 7. Night-light placement list (33 light sources total)

A candle cluster/candelabrum is one emitter regardless of its visible candle count. Use warm candle emission near 1800 K; approximate point/area falloff radius 5–7 m. Moonlight is cool, low-intensity directional/area fill near 7000 K. Keep the total at **29 candle-group emitters + 4 moonlight emitters = 33**, well under the 150-source limit.

| Group | Placement | Count | Emitter guidance |
|---|---|---:|---|
| Narthex wall candle clusters | `x=±2.4, y=5.2, z=3.2 m` (12,26,16 vox) | 2 | One group on each side of main entry; radius ~4 m. |
| Nave side candelabra | per bay `i=0..5`: `x=±8.4, y=12+8i, z=5.2 m` (±42, 60+40i, 26 vox) | 12 | One group on each outer aisle wall per bay; radius ~5 m. |
| Nave hanging clusters | per bay `i=0..5`: `x=0, y=12+8i, z=12 m` (0,60+40i,60 vox) | 6 | One central hanging light group each bay; do not model fine chains—use a 0.4 m thick stone/iron drop bracket or omit suspension. |
| Transept arm candle groups | `x=±11.8, y=44, z=5.2 m` (±59,220,26 vox) and `x=0, y=41/47, z=5.2 m` (0,205/235,26 vox) | 4 | Two on the north/south arms and two at the crossing's west/east ends. |
| Choir wall clusters | `x=±4, y=58, z=4.8 m` (±20,290,24 vox) | 2 | Warm pools toward apse, radius ~4 m. |
| High-altar candle groups | `x=-1.2,0,+1.2; y=62.4; z=1.4 m` (−6/0/+6,312,7 vox) | 3 | Three grouped candle stands; radius ~3 m. |
| Moonlight through apse glass | centre `(0,68,7.4 m)` (0,340,37 vox), direction westward | 1 | Cool area/directional source; narrow coloured transmission through the one glass panel. |
| Moonlight through west rose opening | centre `(0,0,14 m)` (0,0,70 vox), direction eastward | 1 | Cool low-intensity directional fill; rose is open, no coloured glass. |
| North-side moonlight | outside north wall `(-12,32,20 m)` (−60,160,100 vox), direction southward | 1 | Soft directional fill entering the unglazed clerestory apertures. |
| South-side moonlight | outside south wall `(+12,32,20 m)` (+60,160,100 vox), direction northward | 1 | Mirror of north fill. |
| **Total** |  | **33** | 29 candle emitters + 4 moonlight emitters. |

## 8. Fine-detail props (5 cm voxels only; 12 props)

Each prop is a separate detail-resolution asset, never a building-shell exception. Sizes below are **metres and 5 cm prop voxels**; each is under 2 × 2 × 3 m. Positions are minimum corners in metres, then 5 cm voxel coordinates. `x,y,z` still use the world origin above.

| # | Detail prop | Position (m → 5 cm vox) | Size (m → 5 cm vox) | Material / note |
|---:|---|---|---|---|
| 1 | High altar table | `(-0.9,61.4,0)` → `(-18,1228,0)` | `1.8×1.0×1.0` → `36×20×20` | M02 marble slab top, M10 oak underframe. |
| 2 | High-altar retable | `(-0.9,63.2,0)` → `(-18,1264,0)` | `1.8×0.4×2.4` → `36×8×48` | M02; stepped carved panels, no tiny filigree. |
| 3 | Ambo / lectern | `(-4.0,57.2,0)` → `(-80,1144,0)` | `0.8×0.8×1.6` → `16×16×32` | M10 oak, M02 base. |
| 4 | Choir bishop chair | `(-0.6,58.2,0)` → `(-12,1164,0)` | `1.2×0.8×1.8` → `24×16×36` | M10 oak with broad M02 seat rail. |
| 5 | Pulpit | `(-8.8,46.0,0)` → `(-176,920,0)` | `1.2×1.2×2.4` → `24×24×48` | M02 stone, M10 lectern surface. |
| 6 | Wedding kneeler — north | `(-0.9,46.4,0)` → `(-18,928,0)` | `0.6×0.6×0.7` → `12×12×14` | M10 oak; paired with #7. |
| 7 | Wedding kneeler — south | `(+0.3,46.4,0)` → `(6,928,0)` | `0.6×0.6×0.7` → `12×12×14` | M10 oak. Ceremony point centred at `(0,47,0)`. |
| 8 | Processional cross | `(-0.3,47.4,0)` → `(-6,948,0)` | `0.6×0.2×1.8` → `12×4×36` | M09 reflective iron; thick cross arms. |
| 9 | Reliquary chest with bone/moss panel | `(-4.0,62.0,0)` → `(-80,1240,0)` | `1.2×0.6×0.8` → `24×12×16` | M02 chest, M13 bone-with-moss inset. |
| 10 | Paschal candelabrum | `(-0.3,60.8,0)` → `(-6,1216,0)` | `0.6×0.6×2.4` → `12×12×48` | M09 stem, M11 wax + M12 flame; emitter belongs to altar group. |
| 11 | North altar candle stand | `(-1.8,61.8,0)` → `(-36,1236,0)` | `0.4×0.4×1.6` → `8×8×32` | M09 + M11 wax + M12 flame; included in altar candle emitter group. |
| 12 | South altar candle stand | `(+1.4,61.8,0)` → `(28,1236,0)` | `0.4×0.4×1.6` → `8×8×32` | M09 + M11 wax + M12 flame; included in altar candle emitter group. |

**No other statues or hand-authored props are required.** A small number of large block reliefs may be integrated into the marble façade as ordinary 20 cm voxels; do not add separate 5 cm assets unless replacing one of the 12 listed props.

## 9. Suggested camera views

Positions/targets are `(x,y,z)` in metres, followed by base-voxel coordinates (20 cm units). Lens values are optional starting points. Cameras may use sub-voxel interpolation; listed values are aligned to the base grid.

| View | Eye position (m → vox) | Look target (m → vox) | Framing |
|---|---|---|---|
| 1. Wedding, human eye height | `(-2.4,45.2,1.6)` → `(-12,226,8)` | `(0,47,1.4)` → `(0,235,7)` | Near the paired kneelers at 1.6 m eye height; couple/ceremony point in foreground, crossing and altar beyond. |
| 2. Axial nave interior | `(0,4,1.6)` → `(0,20,8)` | `(0,62,5.2)` → `(0,310,26)` | From just inside west entry toward the apse; shows all six bays and the single stained panel. |
| 3. Apse glass detail | `(0,57.2,1.8)` → `(0,286,9)` | `(0,67.6,7.4)` → `(0,338,37)` | Low interior view directly toward the pointed panel; reveals its lead pattern and transmitted moonlight. |
| 4. Wide west exterior | `(24,-12,12)` → `(120,-60,60)` | `(0,8,18)` → `(0,40,90)` | Three-quarter wide establishing view, includes both towers, west façade and plaza. |
| 5. North-side profile | `(-24,34,14)` → `(-120,170,70)` | `(-2,34,15)` → `(-10,170,75)` | Orthographic or long-lens profile; reads bay rhythm, buttresses, aisle roof and nave ridge. |
| 6. Crossing / transept interior | `(-8.4,43.2,1.6)` → `(-42,216,8)` | `(0,44,16)` → `(0,220,80)` | Human-scale upward view from the north aisle edge to the crossing ribs and nave vault. |

## 10. Build order and procedural notes

1. Lay out the plaza plane and all floor zones on the 20 cm grid. Keep the apse floor polygon clipped to its listed vertices.
2. Build narthex, nave perimeter walls, transept arms, choir, and octagonal apse as block masses; cut portal and window voids afterward.
3. Instantiate paired towers and seven pier stations, then 12 buttresses. Place repeated six-bay arches and vaults from the module function.
4. Add roof slabs, stepped spires, stone rose ring, and broad façade bands. The rose is an open aperture with radial stone ribs, never glass.
5. Apply the single apse stained-glass panel and its chunky lead grid. Leave every other listed opening unglazed.
6. Place the 12 5 cm detail props and the scheduled 29 candle clusters + 4 moonlight sources.
7. Apply night exposure: marble receives cool moonlight plus warm local candle pools; stained glass transmits coloured light but does not emit. Keep the maximum architectural height at `z=38 m` (190 voxels).

**Budget checks:** footprint 28 × 68 m (within limits); height 38 m (≤40 m); nave vault crown 22 m (within 20–24 m); 6 repeated bays; 1 stained-glass panel; 12 materials (≤20); 33 emitters (≤150); 12 fine-detail props (≤20).
