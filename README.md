# Diorama Raytracer

**Nombre:** Camila Sandoval  
**Carnet:** #24358

Proyecto de ray tracing en Rust que renderiza un diorama voxelizado de una catedral gótica nocturna, con iluminación de velas y luna, vitrales, reflexión, refracción y una invasión zombi.

## Video del proyecto

[Ver el video de la escena ejecutándose](diorama-raytracer-demo.mp4)

<video controls width="720">
  <source src="diorama-raytracer-demo.mp4" type="video/mp4">
</video>

## Librerías

Se cumplen los requisitos base de no utilizar librerías externas ajenas al curso. Únicamente se utilizan las librerías permitidas y utilizadas en el curso: `minifb`, `nalgebra-glm` e `image`.

## Requisitos cumplidos

- **Complejidad de la escena — 30 puntos:** catedral gótica con nave, naves laterales, crucero, bóvedas, vitrales, props, personajes y zombis.
- **Atractivo visual — 20 puntos:** iluminación nocturna con velas cálidas, luna fría, vitrales coloreados y postprocesado.
- **Rotación y zoom — 20 puntos:** cámara orbital controlada con teclado y mouse, con acercamiento y alejamiento.
- **Cinco materiales — 25 puntos:** piedra/mármol, madera, vidrio, metal y hueso con musgo, cada uno con textura y parámetros propios.
- **Refracción — 10 puntos:** vidrio de los vitrales con transparencia, índice de refracción y transmisión de color.
- **Reflexión — 5 puntos:** metal, hierro, emplomado y superficies pulidas reflectivas.
- **Skybox — 20 puntos:** cielo nocturno procedural con luna y estrellas.
