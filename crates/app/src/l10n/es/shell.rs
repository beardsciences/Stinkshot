//! Spanish strings for the shell group. Missing keys fall back to English. Key names (Ctrl,
//! Shift, PrtSc, Esc) stay in English, as the app shows them everywhere else.

pub fn get(key: &str) -> Option<&'static str> {
    Some(match key {
        "shell.tray.history" => "Historial",
        "shell.tray.settings" => "Configuración",
        "shell.tray.quit" => "Salir",
        "shell.tray.more" => "Más",
        "shell.action.region" => "Región",
        "shell.action.window" => "Ventana",
        "shell.action.fullscreen" => "Pantalla completa",
        "shell.action.scrolling" => "Con desplazamiento",
        "shell.action.ocr" => "Copiar texto (OCR)",
        "shell.action.repeat_last" => "Repetir la última región",
        "shell.action.delayed_region" => "Región con retraso",
        "shell.action.selected_text" => "Capturar el texto seleccionado",
        "shell.action.selected_paragraph" => "Capturar el párrafo seleccionado",
        "shell.action.pick_color" => "Tomar color",
        "shell.action.record_mp4" => "Grabar MP4",
        "shell.action.record_gif" => "Grabar GIF",
        "shell.action.region.desc" => "Congela la pantalla y arrastra una región.",
        "shell.action.window.desc" => "Captura la ventana activa con sus esquinas redondeadas.",
        "shell.action.fullscreen.desc" => "Captura el monitor que está bajo el puntero.",
        "shell.action.scrolling.desc" => {
            "Selecciona una región para desplazarla y capturarla como una sola imagen alta."
        }
        "shell.action.ocr.desc" => "Selecciona una región y copia el texto reconocido.",
        "shell.action.repeat_last.desc" => "Vuelve a capturar la región anterior.",
        "shell.action.delayed_region.desc" => {
            "Espera unos segundos y luego selecciona una región. Mientras tanto, abre el menú o la información sobre herramientas."
        }
        "shell.action.selected_text.desc" => "Captura el texto resaltado en la aplicación activa.",
        "shell.action.selected_paragraph.desc" => {
            "Captura los párrafos que rodean el texto resaltado."
        }
        "shell.selected_text.none" => "No hay texto seleccionado",
        "shell.selected_text.none.body" => "Primero selecciona texto en la aplicación.",
        "shell.action.pick_color.desc" => {
            "Haz clic en cualquier píxel para copiar su color en hex, RGB o HSL."
        }
        "shell.action.record_mp4.desc" => {
            "Selecciona una región y graba con codificación por hardware."
        }
        "shell.action.record_gif.desc" => {
            "Selecciona una región y graba un GIF de tamaño optimizado."
        }
        "shell.error.disk_full" => "El disco está lleno. Libera espacio y vuelve a intentarlo.",
        "shell.error.access_denied" => {
            "Windows bloqueó el acceso al archivo o la carpeta. Revisa sus permisos o elige otra carpeta."
        }
        "shell.error.path_too_long" => {
            "La ruta del archivo es demasiado larga. Elige una carpeta o un patrón de nombre más corto."
        }
        "shell.error.not_found" => {
            "No se encontró el archivo o la carpeta. Puede que se haya movido o eliminado."
        }
        "shell.error.in_use" => {
            "Otro programa está usando el archivo. Ciérralo y vuelve a intentarlo."
        }
        "shell.error.network_path" => {
            "No se puede acceder a la ubicación de red. Revisa la conexión y vuelve a intentarlo."
        }
        "shell.error.clipboard_busy" => {
            "Otra aplicación está usando el portapapeles. Vuelve a intentarlo en un momento."
        }
        "shell.error.offline" => {
            "No se pudo conectar con stinkshot.com. Revisa la conexión y vuelve a intentarlo."
        }
        "shell.error.unsupported" => {
            "Esto todavía no funciona en este sistema. Por ahora solo funciona en Windows."
        }
        "shell.unsupported" => "Todavía no disponible",
        "shell.toast.edit" => "Editar",
        "shell.toast.copy" => "Copiar",
        "shell.toast.pin" => "Fijar",
        "shell.toast.copy_text" => "Copiar texto",
        "shell.toast.folder" => "Mostrar en la carpeta",
        "shell.toast.delete" => "Mover a la Papelera de reciclaje",
        "shell.toast.play" => "Reproducir",
        "shell.toast.edit.tip" => "Abrir en el editor",
        "shell.toast.copy_file.tip" => "Copiar el archivo al portapapeles",
        "shell.toast.copy_image.tip" => "Copiar la imagen al portapapeles",
        "shell.toast.pin.tip" => "Fijar en pantalla",
        "shell.toast.copy_text.tip" => "Copiar el texto reconocido",
        "shell.toast.folder.tip" => "Mostrar en la carpeta",
        "shell.toast.delete.tip" => "Mover a la Papelera de reciclaje",
        "shell.toast.dismiss" => "Descartar",
        "shell.toast.ocr_missing" => "Requiere un idioma con reconocimiento de texto en Windows",
        "shell.tray.update_check_on" => "Búsqueda de actualizaciones activada",
        "shell.tray.stop_recording" => "Detener la grabación",
        "shell.tray.offline" => "Sin conexión",
        "shell.pin.copied" => "Copiado",
        "shell.pin.copy_failed" => "No se pudo copiar",
        "shell.pin.copy_image" => "Copiar imagen",
        "shell.pin.open_editor" => "Abrir en el editor",
        "shell.pin.reset_zoom" => "Restablecer el zoom al 100%",
        "shell.pin.close" => "Cerrar la captura fijada",
        "shell.pin.opacity.tip" => "Opacidad (Ctrl+rueda)",
        "shell.pin.copy.tip" => "Copiar (Ctrl+C)",
        "shell.pin.close.tip" => "Cerrar la captura fijada (Esc)",
        "shell.pin.edit.tip" => "Abrir en el editor (doble clic)",
        "shell.pin.window_title" => "Fijada: {name}",
        "shell.notice.no_last_region" => "Primero selecciona una región",
        "shell.notice.last_region_off_screen" => "La última región está fuera de la pantalla",
        "shell.error.window_capture" => {
            "Windows no pudo capturar esa ventana. Vuelve a intentarlo o usa la captura de región."
        }
        "shell.start.retry" => "Intenta iniciarlo de nuevo.",
        "shell.welcome.no_key" => {
            "Captura desde el icono de Stinkshot en la bandeja. Abre Stinkshot otra vez para ver el historial."
        }
        "shell.hud.mic_system" => "Mic + sistema",
        "shell.hud.mic" => "Mic",
        "shell.hud.system_audio" => "Audio del sistema",
        "shell.hud.no_audio" => "Sin audio",
        "shell.hotkeys.inactive" => "Algunos atajos no están activos",
        "shell.history_dir.fallback" => "Carpeta del historial no disponible",
        "shell.history_dir.fallback.body" => {
            "No se pudo abrir {dir}. Las capturas van a la carpeta predeterminada hasta que vuelva."
        }
        "shell.tray.unavailable" => "Icono de la bandeja no disponible",
        "shell.tray.unavailable.body" => {
            "Stinkshot sigue abierto. Usa los atajos o la línea de comandos."
        }
        "shell.welcome.title" => "Stinkshot está abierto",
        "shell.star.failed.body" => {
            "No se pudo actualizar el índice del historial. Vuelve a intentarlo en un momento."
        }
        "shell.star.failed" => "No se pudo actualizar la marca de destacada",
        "shell.busy.recording" => "Grabación en curso",
        "shell.busy.recording.capture" => "Primero termina la grabación.",
        "shell.busy.capture_open" => "Primero termina la captura actual.",
        "shell.busy.capture" => "Primero termina la captura actual.",
        "shell.busy.recording.edit" => "Primero termina la grabación.",
        "shell.capture.no_image.body" => {
            "Windows no devolvió la imagen de la pantalla. Vuelve a intentarlo o reinicia Stinkshot si sigue fallando."
        }
        "shell.capture.screen_failed" => "No se pudo capturar la pantalla",
        "shell.capture.no_window" => "No hay ninguna ventana que capturar",
        "shell.capture.no_window.body" => "Primero haz clic en la ventana y vuelve a intentarlo.",
        "shell.capture.no_monitor.body" => {
            "Ese monitor no estaba en la imagen de la pantalla. Mueve el puntero a él y vuelve a intentarlo."
        }
        "shell.ocr.no_text" => "No se encontró texto",
        "shell.ocr.empty" => "La selección estaba vacía.",
        "shell.selection.lost" => "No se pudo leer la selección",
        "shell.selection.lost.body" => "Se perdió la imagen de la pantalla. Vuelve a intentarlo.",
        "shell.selection.too_small" => "Selección demasiado pequeña",
        "shell.selection.too_small.scroll" => {
            "Selecciona un área más grande para la captura con desplazamiento."
        }
        "shell.selection.too_small.record" => "Selecciona un área más grande para grabar.",
        "shell.busy.recording.record" => "Primero termina la grabación.",
        "shell.record.temp_failed.body" => {
            "La carpeta del historial no está lista para grabar. Revísala en Configuración > Historial."
        }
        "shell.record.start_failed" => "No se pudo empezar a grabar",
        "shell.record.problem.body" => "La grabación se interrumpió.",
        "shell.record.problem.tail" => "Se detiene y conserva lo grabado.",
        "shell.record.problem" => "Problema de grabación",
        "shell.record.disk_low" => "Disco casi lleno",
        "shell.record.disk_low.body" => "La grabación se detuvo para conservar lo grabado.",
        "shell.capture.window_failed" => "No se pudo capturar la ventana",
        "shell.scroll.stopped" => "La captura con desplazamiento se detuvo antes de tiempo",
        "shell.scroll.stopped.body" => "La página dejó de coincidir. Se conserva lo capturado.",
        "shell.scroll.failed.body" => {
            "No se pudo capturar nada de esa área. Vuelve a intentarlo con la página desplazada hasta arriba."
        }
        "shell.scroll.failed" => "No se pudo terminar la captura con desplazamiento",
        "shell.record.finish_failed.body" => {
            "No se pudo terminar el archivo de grabación. Vuelve a grabar."
        }
        "shell.record.finish_failed" => "No se pudo terminar la grabación",
        "shell.record.recovered" => "Grabación recuperada",
        "shell.record.recovered.body" => "Se agregó al historial una grabación sin guardar.",
        "shell.record.at_60" => "Grabando a 60 FPS",
        "shell.record.mic_failed" => "No se agregó el micrófono",
        "shell.record.mic_failed.body" => "No se pudo abrir. La grabación sigue sin él.",
        "shell.record.start_failed.body" => {
            "Windows no pudo iniciar la grabadora de pantalla. Prueba un área más pequeña o reinicia Stinkshot."
        }
        "shell.export.failed.body" => "No se pudo exportar la grabación.",
        "shell.export.failed.tail" => {
            "La ventana de recorte sigue abierta. Vuelve a presionar Guardar."
        }
        "shell.export.failed" => "No se pudo exportar la grabación",
        "shell.export.partial.body" => "Parte de la exportación falló.",
        "shell.export.partial.tail" => "Lo guardado está en el historial.",
        "shell.saved.mp4_gif" => "MP4 y GIF guardados",
        "shell.saved.gif" => "GIF guardado",
        "shell.saved.recording" => "Grabación guardada",
        "shell.pin.default_title" => "Captura fijada",
        "shell.saved.text" => "Texto copiado",
        "shell.saved.updated" => "Captura actualizada",
        "shell.saved.copy" => "Guardada como copia",
        "shell.saved.pinned" => "Fijada en pantalla",
        "shell.saved.copied" => "Copiada al portapapeles",
        "shell.saved.history" => "Guardada en el historial",
        "shell.update.store" => "Las actualizaciones llegan desde Microsoft Store",
        "shell.update.store.body" => "Abre Microsoft Store para buscar una versión más reciente.",
        "shell.update.off" => "La búsqueda de actualizaciones está desactivada",
        "shell.update.off.body" => {
            "Actívala en Configuración > Privacidad. Es la única solicitud de red."
        }
        "shell.update.available.body" => "Descárgala desde stinkshot.com.",
        "shell.update.current" => "Stinkshot está actualizado",
        "shell.update.untrusted" => {
            "La información de la versión no superó la comprobación de firma, así que se ignoró."
        }
        "shell.update.unsigned" => {
            "La última versión aún no tiene información firmada. Vuelve a intentarlo más tarde."
        }
        "shell.update.no_answer" => {
            "El servidor de actualizaciones no respondió. Vuelve a intentarlo más tarde."
        }
        "shell.update.failed" => "No se pudo buscar actualizaciones",
        "shell.update.available.install" => "Actualizar ahora la instala y reinicia Stinkshot.",
        "shell.update.downloading" => "Descargando Stinkshot {tag}",
        "shell.update.downloading.body" => "Stinkshot se reinicia en la versión nueva al terminar.",
        "shell.update.install_failed" => "No se pudo actualizar",
        "shell.update.install_stopped" => "El instalador se cerró antes de terminar.",
        "shell.update.install_failed.body" => "La descarga no terminó.",
        "shell.update.install_untrusted" => {
            "La descarga no coincidía con la información firmada de la versión, así que se eliminó."
        }
        "shell.update.opening_page" => "Se abre la página de descarga en su lugar.",
        "shell.update.updated" => "Actualizado a {v}",
        "shell.update.updated.body" => "Se conservan tu configuración, tus atajos y el historial.",
        "shell.tray.update_now" => "Actualizar a {tag}",
        "shell.toast.update_now" => "Actualizar ahora",
        "shell.toast.update_now.tip" => "Descargar, instalar y reiniciar",
        "shell.toast.download" => "Descargar",
        "shell.toast.download.tip" => "Abrir la página de descarga",
        "shell.toast.whats_new" => "Novedades",
        "shell.toast.whats_new.tip" => "Abrir las notas de la versión",
        "shell.config.save_failed.body" => "No se pudo escribir el archivo de configuración.",
        "shell.config.save_failed.tail" => {
            "Tus cambios se aplican hasta que Stinkshot se reinicie."
        }
        "shell.config.save_failed" => "No se pudo guardar la configuración",
        "shell.history_dir.failed.body" => "Stinkshot no pudo usar esa carpeta.",
        "shell.history_dir.failed.tail" => "El historial se queda en la carpeta anterior.",
        "shell.history_dir.failed" => "No se pudo abrir esa carpeta de historial",
        "shell.start.failed" => "Stinkshot no pudo iniciarse. {why}",
        "shell.welcome.key" => {
            "Presiona {key} para capturar una región o usa el icono de la bandeja. Abre Stinkshot otra vez para ver el historial."
        }
        "shell.hud.gif" => "GIF · {fps} FPS · Hasta {w} px de ancho · Sin audio",
        "shell.hud.mp4" => "MP4 · {fps} FPS · {audio}",
        "shell.record.needs_hw" => "{fps} FPS requiere codificación por hardware.",
        "shell.saved.as" => "Se guardó {name}",
        "shell.update.available" => "Stinkshot {tag} está disponible",
        "shell.update.current.body" => "La versión {v} es la más reciente.",
        "shell.tray.update_available" => "Actualización disponible: {tag}",
        "shell.pin.ready" => "{name} está listo para pegar.",
        "shell.pin.opacity" => "Opacidad {pct}%",
        "shell.error.log_hint" => {
            "Los detalles están en el registro (Configuración > Acerca de > Mostrar registro)."
        }
        "shell.countdown.cancel" => "Esc para cancelar",
        "shell.hud.stop_trim" => "Detener y recortar",
        "shell.hud.resume" => "Reanudar",
        "shell.hud.pause" => "Pausar",
        "shell.hud.microphone" => "Micrófono",
        "shell.hud.webcam" => "Cámara web",
        "shell.hud.show_input" => "Mostrar clics y teclas",
        "shell.hud.stop" => "Detener",
        "shell.hud.problem" => "Problema de grabación · Guardando",
        "shell.trim.discard_prompt" => "¿Descartar esta grabación?",
        "shell.trim.discard_prompt.body" => "Aún no se ha guardado. No se puede deshacer.",
        "shell.trim.save_prompt" => "¿Guardar esta grabación?",
        "shell.trim.title" => "Recortar la grabación",
        "shell.trim.loading" => "Cargando la vista previa",
        "shell.trim.play" => "Reproducir (Espacio)",
        "shell.trim.pause" => "Pausar (Espacio)",
        "shell.trim.reading" => "Leyendo fotogramas",
        "shell.trim.gif_warning" => "Los clips largos generan GIF muy grandes. MP4 ocupa menos.",
        "shell.trim.discard" => "Descartar",
        "shell.trim.keep_editing" => "Seguir editando",
        "shell.trim.save_untrimmed" => "Guardar sin recortar",
        "shell.scroll.cancel" => "Cancelar",
        "shell.scroll.stop_here" => "Detener aquí",
        "shell.hud.off_in_settings" => "{name}: desactivado en Configuración > Grabación",
        "shell.hud.on" => "{name}: activado",
        "shell.hud.off" => "{name}: desactivado",
        "shell.hud.stop_trim_key" => "Detener y recortar ({key})",
        "shell.hud.dropped" => "{summary} \u{b7} {n} fotogramas omitidos",
        "shell.trim.save_prompt.body" => {
            "Aún no se ha guardado. Guardar sin recortar conserva la grabación entera como {fmt}."
        }
        "shell.trim.keep" => "Conservar {t}",
        "shell.scroll.stitching" => "Capturando pantalla {n}",
        "shell.internal.title" => "Algo salió mal",
        "shell.clipboard.busy_saved" => {
            "Otra aplicación está usando el portapapeles. La captura se guardó. Usa Copiar en la notificación o en el historial."
        }
        "shell.clipboard.refused_saved" => {
            "Windows rechazó la imagen en el portapapeles. La captura está en el historial. Prueba Copiar allí."
        }
        "shell.clipboard.refused" => {
            "Windows no lo aceptó en el portapapeles. Vuelve a intentarlo en un momento."
        }
        "shell.item.unreadable" => {
            "No se pudo leer el archivo. Puede que se haya movido o eliminado."
        }
        "shell.save.history_failed.body" => {
            "No se pudo escribir la captura en la carpeta del historial."
        }
        "shell.save.history_failed" => "No se pudo guardar en el historial",
        "shell.save.history_failed.elsewhere" => "Se guardó en {dir} en su lugar.",
        "shell.save.history_failed.clipboard" => "Se copió al portapapeles en su lugar.",
        "shell.save.history_failed.lost" => "La captura no se conservó.",
        "shell.saved.default_folder" => "Guardada en la carpeta predeterminada",
        "shell.history_dir.changed" => "Carpeta del historial cambiada",
        "shell.history_dir.changed.body" => {
            "Las capturas de {dir} se quedan allí y ya no aparecen en el historial."
        }
        "shell.history_dir.back" => "Carpeta del historial disponible",
        "shell.history_dir.back.body" => {
            "{dir} vuelve a estar conectada. Las capturas van allí de nuevo."
        }
        "shell.end_session.reason" => "Guardando la grabación",
        "shell.crash.body" => {
            "Stinkshot sufrió un error interno y se cerró. Los detalles están en el registro: {log}"
        }
        "shell.discard.quit" => {
            "Una ventana del editor tiene cambios sin guardar. ¿Salir y descartarlos?"
        }
        "shell.discard.update" => {
            "Una ventana del editor tiene cambios sin guardar. ¿Actualizar ahora y descartarlos?"
        }
        "shell.clipboard.failed" => "No se pudo copiar al portapapeles",
        "shell.save_as.failed" => "No se pudo guardar el archivo",
        "shell.save_as.failed.body" => {
            "No se pudo escribir el archivo ahí. Prueba con otra carpeta."
        }
        "shell.save_as.open_failed" => "No se pudo abrir Guardar como",
        "shell.save_as.open_failed.body" => "La captura sigue en el historial.",
        "shell.ocr.nothing" => "No hay nada legible en la selección.",
        "shell.ocr.copy_failed" => "No se pudo copiar el texto",
        "shell.ocr.failed" => "No se pudo reconocer el texto",
        "shell.ocr.failed.capture" => {
            "Windows no pudo leer texto en la captura. Prueba con una selección más grande o más nítida."
        }
        "shell.ocr.failed.selection" => {
            "Windows no pudo leer texto en la selección. Prueba con una selección más grande o más nítida."
        }
        "shell.item.image_copied" => "Imagen copiada",
        "shell.item.file_copied" => "Archivo copiado",
        "shell.item.ready" => "Listo para pegar.",
        "shell.ocr.item_none" => "Este elemento no tiene texto reconocible.",
        "shell.item.pin_failed" => "No se pudo fijar",
        "shell.item.pin_failed.body" => {
            "No se pudo cargar la imagen. Puede que se haya movido o eliminado."
        }
        "shell.item.edit_failed" => "No se pudo abrir para editar",
        "shell.item.canvas_failed" => "No se pudo agregar al lienzo",
        "shell.canvas.name" => "Lienzo",
        "shell.item.edit_failed.body" => {
            "No se pudo cargar la imagen. Puede que se haya movido o eliminado."
        }
        "shell.item.in_use" => "Cierra los programas que lo tengan abierto y vuelve a intentarlo.",
        "shell.item.delete_failed" => "No se pudo eliminar",
        "shell.item.forget_failed.body" => {
            "El archivo está en la Papelera de reciclaje pero sigue en la lista. Reinicia Stinkshot para actualizar el historial."
        }
        "shell.item.open_failed.body" => {
            "Windows no pudo abrirlo. Revisa que el archivo todavía existe."
        }
        "shell.item.open_failed" => "No se pudo abrir",
        "shell.item.not_found" => "No se encontró ese elemento",
        "shell.item.not_found.body" => "Puede que se haya movido o eliminado.",
        "shell.history.read_failed.body" => {
            "No se pudo leer el índice del historial. Reinicia Stinkshot y vuelve a intentarlo."
        }
        "shell.history.read_failed" => "No se pudo leer el historial",
        "shell.url.failed.body" => {
            "Windows no pudo abrir el navegador. Revisa que hay un navegador predeterminado."
        }
        "shell.url.failed" => "No se pudo abrir la página",
        "shell.internal.body" => {
            "Stinkshot tuvo un error interno y detuvo esa tarea. Vuelve a intentarlo. {hint}"
        }
        "shell.ocr.none" => "No hay ningún idioma con reconocimiento de texto",
        "shell.ocr.unavailable" => "Reconocimiento de texto no disponible",
        "shell.ocr.none.body" => {
            "Agrega un idioma con reconocimiento de texto en Configuración de Windows > Hora e idioma > Idioma y región."
        }
        "shell.item.recycle_failed" => {
            "Windows no pudo moverlo a la Papelera de reciclaje, así que se conservó. {why}"
        }
        "shell.hotkey.row.invalid" => "No es una combinación de teclas válida. Vuelve a grabarla.",
        "shell.hotkey.row.duplicate" => {
            "También está asignado a otra acción, así que solo funciona una de ellas."
        }
        "shell.hotkey.row.in_use" => "Ya lo usa otro programa. Ciérralo o elige otra tecla.",
        "shell.hotkey.row.not_registered" => "Windows no pudo registrarlo. Elige otra tecla.",
        "shell.hotkey.unavailable" => {
            "Los atajos globales no están disponibles. Usa el icono de la bandeja o reinicia Stinkshot."
        }
        "shell.hotkey.prtsc_failed" => {
            "No se pudo usar PrtSc. Puede que siga abriendo la Herramienta Recortes. Desactiva Usar la tecla PrtSc o vuelve a intentarlo."
        }
        "shell.hotkey.snip_thread_failed" => {
            "Stinkshot no pudo iniciar su detector de atajos. Reinicia Stinkshot y vuelve a intentarlo."
        }
        "shell.hotkey.snip_failed" => {
            "Win+Shift+S y Win+Shift+R siguen abriendo la Herramienta Recortes. Desactiva y activa Reemplazar la Herramienta Recortes, o reinicia Stinkshot."
        }
        "shell.hotkey.not_set" => "Sin asignar",
        "shell.toggle.on" => "Activado",
        "shell.toggle.off" => "Desactivado",
        "shell.about.tagline" => "Capturas y grabación de pantalla privadas para Windows",
        "shell.hotkey.phrase.invalid" => "no es una combinación de teclas válida",
        "shell.hotkey.phrase.duplicate" => "está asignado a la vez a",
        "shell.hotkey.phrase.in_use" => "ya lo usa otro programa",
        "shell.hotkey.phrase.not_registered" => "no se pudo registrar en Windows",
        "shell.hotkey.invalid" => {
            "{key} ({action}) {phrase}. Vuelve a grabarlo en Configuración > Atajos de teclado."
        }
        "shell.hotkey.duplicate" => {
            "{key} {phrase} {other} y {action}. Solo funciona {other}. Cambia uno en Configuración > Atajos de teclado."
        }
        "shell.hotkey.in_use" => {
            "{key} ({action}) {phrase}. Cierra ese programa o elige otra tecla en Configuración > Atajos de teclado."
        }
        "shell.hotkey.not_registered" => {
            "{key} ({action}) {phrase}. Elige otra tecla en Configuración > Atajos de teclado."
        }
        "shell.about.version" => "Versión {v}",
        "shell.save_as.png" => "Imagen PNG",
        "shell.save_as.jpeg" => "Imagen JPEG",
        "shell.save_as.webp" => "Imagen WebP",
        "shell.save_as.title" => "Guardar captura como",
        "shell.trim.save_gif" => "Guardar GIF",
        "shell.trim.save_mp4" => "Guardar MP4",
        "shell.tray.help" => "Ayuda",
        "shell.tour.skip" => "Omitir recorrido",
        "shell.tour.back" => "Atrás",
        "shell.tour.next" => "Siguiente",
        "shell.tour.done" => "Listo",
        "shell.tour.open_help" => "Abrir la ayuda",
        "shell.tour.hotkeys.title" => "Captura con un atajo",
        "shell.tour.hotkeys.body" => {
            "Presiona {key} para capturar una región. Luego dibuja, copia o guarda."
        }
        "shell.tour.hotkeys.body_unset" => {
            "Asigna un atajo de región en Configuración > Atajos de teclado o usa el icono de la bandeja."
        }
        "shell.tour.tray.title" => "Vive en la bandeja",
        "shell.tour.tray.body" => "Stinkshot funciona en segundo plano, junto al reloj.",
        "shell.tour.privacy.title" => "Tus capturas nunca salen de tu PC",
        "shell.tour.privacy.body" => "Stinkshot funciona sin conexión a internet.",
        "shell.tour.privacy.captures" => "Capturas",
        "shell.tour.privacy.captures.what" => "Se quedan en este PC",
        "shell.tour.privacy.ocr" => "Reconocimiento de texto",
        "shell.tour.privacy.ocr.what" => "Funciona sin conexión en Windows",
        "shell.tour.privacy.accounts" => "Cuentas y telemetría",
        "shell.tour.privacy.accounts.what" => "Ninguna",
        "shell.tour.privacy.updates" => "Búsqueda de actualizaciones",
        "shell.tour.privacy.updates.what" => "Desactivada hasta que la actives",
        "shell.tour.files.title" => "Dónde van las capturas",
        "shell.tour.files.body" => {
            "Se guardan aquí y aparecen en el historial. Cámbialo en Configuración > Historial."
        }
        "shell.a11y.item" => "{source}, {w} por {h}, {date}",
        _ => return None,
    })
}
