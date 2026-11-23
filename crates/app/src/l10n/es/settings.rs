//! Spanish strings for settings_ui.rs. Missing keys fall back to English.

pub fn get(key: &str) -> Option<&'static str> {
    Some(match key {
        "settings.language" => "Idioma",
        "settings.language.desc" => "Menús y mensajes.",
        "settings.language.system" => "Predeterminado del sistema",
        "settings.theme" => "Tema",
        "settings.theme.desc" => "Igual que Windows, o siempre oscuro o claro.",
        "settings.nav.general" => "General",
        "settings.nav.hotkeys" => "Atajos de teclado",
        "settings.nav.capture" => "Captura",
        "settings.nav.editor" => "Editor",
        "settings.nav.recording" => "Grabación",
        "settings.nav.history" => "Historial",
        "settings.nav.privacy" => "Privacidad",
        "settings.nav.about" => "Acerca de",
        "settings.custom" => "Personalizado",
        "settings.open_history_tip" => "Abrir historial (Ctrl+H)",
        "settings.search.hint" => "Buscar opciones",
        "settings.search.clear" => "Borrar búsqueda",
        "settings.search.none" => "Ninguna opción coincide",
        "settings.search.kw.general" => "preferencias opciones inicio apariencia",
        "settings.search.kw.hotkeys" => "atajos teclado teclas accesos directos combinaciones",
        "settings.search.kw.capture" => "captura de pantalla recorte",
        "settings.search.kw.editor" => "anotar anotaciones marcas dibujo",
        "settings.search.kw.recording" => "video grabación de pantalla",
        "settings.search.kw.history" => "biblioteca capturas",
        "settings.search.kw.privacy" => "seguridad red",
        "settings.search.kw.about" => "versión información",
        "settings.fps" => "{n} FPS",
        "settings.hotkeys.snip_turning_off" => "Desactivando\u{2026}",
        "settings.hotkeys.snip_turning_on" => "Activando\u{2026}",
        "settings.hotkeys.snip_active" => "Activo",
        "settings.hotkeys.prtsc_unbound" => {
            "Ningún atajo usa PrtSc, así que la Herramienta Recortes la conserva."
        }
        "settings.hotkeys.chord_snip" => "Recorte (Herramienta Recortes)",
        "settings.hotkeys.chord_record" => "Grabación (Herramienta Recortes)",
        "settings.hotkeys.snipping_tool" => "Herramienta Recortes",
        "settings.hotkeys.also_used" => "También lo usa: {list}.",
        "settings.hotkeys.press_keys" => "Presiona las teclas\u{2026}",
        "settings.hotkeys.check_again" => "Comprobar de nuevo",
        "settings.hotkeys.still_in_use" => "Sigue en uso.",
        "settings.hotkeys.no_change" => "Sin cambios.",
        "settings.hotkeys.in_use_many" => {
            "Otro programa usa {n} atajos. Ciérralo o elige otras teclas."
        }
        "settings.capture.example_title" => "Página de ejemplo",
        "settings.capture.pattern.desc" => {
            "Marcadores: {yyyy} {MM} {dd} {HH} {mm} {ss} {fff} {app} {title} {w} {h}. Ejemplo: {preview}"
        }
        "settings.recording.device_missing" => "{name} (no encontrado)",
        "settings.history.forever" => "Siempre",
        "settings.history.no_limit" => "Sin límite",
        "settings.history.one_day" => "1 día",
        "settings.history.days" => "{n} días",
        "settings.history.one_week" => "1 semana",
        "settings.history.one_month" => "1 mes",
        "settings.history.months" => "{n} meses",
        "settings.history.one_year" => "1 año",
        "settings.history.gb" => "{gb} GB",
        "settings.history.folder.desc" => "{path}",
        "settings.history.ocr.desc" => "Funciona sin conexión, en segundo plano.",
        "settings.history.ocr.add_language" => "Agregar un idioma",
        "settings.history.ocr.desc_missing" => {
            "Requiere un idioma de Windows con reconocimiento de texto."
        }
        "settings.history.stats" => "{count} elementos, {size} en uso.",
        "settings.privacy.never_checked" => "Sin comprobar",
        "settings.privacy.just_now" => "hace un momento",
        "settings.privacy.min_ago" => "hace {n} min",
        "settings.privacy.h_ago" => "hace {n} h",
        "settings.privacy.days_ago" => "hace {n} días",
        "settings.privacy.last_checked" => "Última comprobación: {ago}",
        "settings.privacy.check_requested" => "Comprobando. Un mensaje mostrará el resultado.",
        "settings.privacy.stored_in" => "La configuración se guarda en {path}.",
        "settings.about.log_failed.body" => "Vuelve a intentarlo en un momento.",
        "settings.open_history" => "Abrir historial",
        "settings.general.start_with_windows" => "Iniciar con Windows",
        "settings.general.start_with_windows.desc" => "Se inicia en la bandeja al iniciar sesión.",
        "settings.theme.dark" => "Oscuro",
        "settings.theme.light" => "Claro",
        "settings.theme.system" => "Predeterminado del sistema",
        "settings.general.accent" => "Color de énfasis",
        "settings.general.accent.desc" => {
            "Botones, selección, la barra de grabación y las ondas de los clics."
        }
        "settings.general.windows_accent" => "Énfasis de Windows",
        "settings.general.windows_accent.tip" => "Usar el color de énfasis de Windows",
        "settings.general.after_capture" => "Después de una captura",
        "settings.general.copy" => "Copiar al portapapeles",
        "settings.general.copy.desc" => "Cada captura terminada.",
        "settings.general.toast" => "Mostrar notificación",
        "settings.general.toast.desc" => {
            "Una tarjeta pequeña con acciones rápidas después de cada captura."
        }
        "settings.general.toast_duration" => "Duración de la notificación",
        "settings.general.toast_duration.desc" => "Pasar el puntero por encima pausa el tiempo.",
        "settings.general.sound" => "Reproducir sonido",
        "settings.general.hide_capture" => "Ocultar Stinkshot en las capturas",
        "settings.general.hide_capture.desc" => {
            "El editor, las notificaciones y la barra de grabación, también en las grabaciones."
        }
        "settings.general.sound.desc" => "Un breve sonido de obturador al capturar.",
        "settings.general.pins" => "Capturas fijadas",
        "settings.general.pin_task_view" => "Mostrar fijadas en la Vista de tareas",
        "settings.general.pin_task_view.desc" => {
            "Muestra las capturas fijadas en Alt+Tab, la Vista de tareas y la barra de tareas."
        }
        "settings.general.pin_toolbar" => "Mostrar siempre la barra de herramientas",
        "settings.general.pin_toolbar.desc" => "Desactivado: la barra aparece al pasar el puntero.",
        "settings.hotkeys.integration" => "Integración con Windows",
        "settings.hotkeys.replace_snip" => "Reemplazar la Herramienta Recortes",
        "settings.hotkeys.replace_snip.desc" => "Win+Shift+S, Win+Shift+R y PrtSc abren Stinkshot.",
        "settings.hotkeys.printscreen" => "Usar la tecla PrtSc",
        "settings.hotkeys.printscreen.locked" => "Lo controla Reemplazar la Herramienta Recortes.",
        "settings.hotkeys.printscreen.desc" => "PrtSc abre Stinkshot.",
        "settings.hotkeys.shortcuts" => "Acciones",
        "settings.hotkeys.clear" => "Quitar",
        "settings.hotkeys.chord_fixed" => "Lo define Windows.",
        "settings.hotkeys.help" => {
            "Haz clic en un atajo y presiona las teclas. Esc cancela, Backspace lo quita."
        }
        "settings.hotkeys.reset" => "Restablecer atajos",
        "settings.capture.cursor" => "Incluir el puntero del mouse",
        "settings.capture.cursor_object" => "Puntero movible",
        "settings.capture.cursor_object.desc" => "Muévelo o elimínalo en el editor.",
        "settings.capture.snap" => "Ajustar a ventanas y elementos",
        "settings.capture.snap.desc" => {
            "Haz clic en una ventana o elemento resaltado para capturarlo."
        }
        "settings.capture.loupe" => "Lupa",
        "settings.capture.loupe.desc" => {
            "Coordenadas y color bajo el puntero mientras seleccionas."
        }
        "settings.capture.open_editor" => "Editar capturas de ventana y de pantalla completa",
        "settings.capture.open_editor.desc" => {
            "Desactivado: se guardan de inmediato. Usa Editar en la notificación para anotar."
        }
        "settings.capture.after_region" => "Después de seleccionar una región",
        "settings.capture.after_region.desc" => "Ctrl+C siempre copia.",
        "settings.capture.after_region.annotate" => "Anotar sin abrir ventana",
        "settings.capture.after_region.window" => "Abrir el editor con Enter",
        "settings.capture.after_region.instant" => "Abrir el editor de inmediato",
        "settings.capture.window" => "Captura de ventana",
        "settings.capture.corners" => "Esquinas redondeadas",
        "settings.capture.corners.desc" => {
            "Conserva las esquinas reales de Windows 11, transparentes en PNG."
        }
        "settings.capture.shadow" => "Sombra",
        "settings.capture.frame" => "Marco con fondo",
        "settings.capture.frame.desc" => {
            "Coloca la ventana sobre un fondo de color para compartirla."
        }
        "settings.capture.padding" => "Margen",
        "settings.capture.padding.desc" => "Espacio alrededor de la ventana.",
        "settings.capture.padding_color" => "Color del margen",
        "settings.capture.saving" => "Al guardar",
        "settings.capture.format" => "Formato de imagen",
        "settings.capture.format.desc" => {
            "PNG conserva la transparencia y el texto nítido. JPEG y WebP ocupan menos."
        }
        "settings.capture.jpeg_quality" => "Calidad de JPEG",
        "settings.capture.jpeg_quality.desc" => "Más alta ocupa más y se ve más limpia.",
        "settings.capture.pattern" => "Patrón del nombre de archivo",
        "settings.capture.scrolling" => "Captura con desplazamiento",
        "settings.capture.max_height" => "Altura máxima",
        "settings.capture.max_height.desc" => "La captura se detiene a esta altura.",
        "settings.capture.delay" => "Región con retraso",
        "settings.capture.delay.desc" => "Segundos de espera antes de congelar la pantalla.",
        "settings.capture.scroll_from_top" => "Empezar desde arriba",
        "settings.capture.scroll_from_top.desc" => {
            "Primero sube hasta arriba, para capturar la página entera."
        }
        "settings.capture.scroll_delay" => "Espera mínima entre desplazamientos",
        "settings.capture.scroll_delay.desc" => {
            "Si una página está cargando, la espera se alarga sola."
        }
        "settings.editor.defaults" => "Valores predeterminados",
        "settings.editor.color" => "Color inicial",
        "settings.editor.color.desc" => {
            "Cada herramienta recuerda su último color. Cambiar este los restablece todos."
        }
        "settings.editor.stroke" => "Grosor del trazo",
        "settings.editor.stroke.desc" => "Líneas, flechas y formas.",
        "settings.editor.font_size" => "Tamaño de fuente",
        "settings.editor.font_size.desc" => "Anotaciones de texto.",
        "settings.editor.blur" => "Radio de desenfoque",
        "settings.editor.blur.desc" => "Cuanto más alto, más oculta.",
        "settings.editor.pixelate" => "Tamaño del pixelado",
        "settings.editor.pixelate.desc" => "Cuanto más grandes los bloques, más ocultan.",
        "settings.editor.palette" => "Paleta",
        "settings.editor.palette_colors" => "Colores de la paleta",
        "settings.editor.palette_colors.desc" => {
            "Haz clic en un color para editarlo. Clic derecho para quitarlo. Hasta 12 colores."
        }
        "settings.editor.manage" => "Administrar la paleta",
        "settings.editor.manage.desc" => "Agrega, quita o restaura los colores predeterminados.",
        "settings.editor.add" => "Agregar color",
        "settings.editor.remove_last" => "Quitar el último",
        "settings.editor.reset" => "Restablecer",
        "settings.recording.video" => "Video",
        "settings.recording.fps" => "Fotogramas por segundo",
        "settings.recording.fps.desc" => "120 FPS requiere codificación por hardware.",
        "settings.recording.bitrate" => "Calidad",
        "settings.recording.bitrate.desc" => "Más alta se ve más nítida y ocupa más.",
        "settings.recording.quality.standard" => "Estándar",
        "settings.recording.quality.high" => "Alta",
        "settings.recording.quality.max" => "Máxima",
        "settings.recording.mbps" => "{n} Mbps",
        "settings.recording.countdown" => "Cuenta regresiva",
        "settings.recording.countdown.desc" => {
            "Segundos antes de empezar a grabar. Con 0 empieza de inmediato."
        }
        "settings.recording.clicks" => "Mostrar clics del mouse",
        "settings.recording.clicks.desc" => "Ondas que se expanden alrededor de los clics.",
        "settings.recording.keys" => "Mostrar teclas presionadas",
        "settings.recording.audio_webcam" => "Audio y cámara web",
        "settings.recording.system_audio" => "Audio del sistema",
        "settings.recording.system_audio.desc" => "Graba lo que reproduce tu PC.",
        "settings.recording.mic" => "Micrófono",
        "settings.recording.mic.desc" => "Agrega tu micrófono a la grabación.",
        "settings.recording.default_device" => "Dispositivo predeterminado",
        "settings.recording.mic_device" => "Dispositivo de micrófono",
        "settings.recording.mic_device.desc" => "Dispositivo predeterminado usa el de Windows.",
        "settings.recording.scanning" => "Buscando dispositivos\u{2026}",
        "settings.recording.webcam" => "Burbuja de cámara web",
        "settings.recording.webcam.desc" => "Muestra la cámara web en un círculo en la grabación.",
        "settings.recording.webcam_device" => "Dispositivo de cámara web",
        "settings.recording.webcam_device.desc" => "Qué cámara mostrar.",
        "settings.recording.webcam_size" => "Tamaño de la cámara web",
        "settings.recording.devices" => "Dispositivos",
        "settings.recording.refresh" => "Actualizar dispositivos",
        "settings.recording.gif" => "Salida GIF",
        "settings.recording.gif_fps" => "FPS del GIF",
        "settings.recording.gif_fps.desc" => "Menos ocupa menos.",
        "settings.recording.gif_width" => "Ancho máximo del GIF",
        "settings.recording.gif_width.desc" => "Las grabaciones más anchas se reducen.",
        "settings.history.folder" => "Carpeta del historial",
        "settings.history.use_default" => "Usar la predeterminada",
        "settings.open" => "Abrir",
        "settings.history.choosing" => "Eligiendo\u{2026}",
        "settings.history.change" => "Cambiar",
        "settings.history.open_failed" => "No se pudo abrir la carpeta",
        "settings.history.open_failed.hint" => {
            "Windows no pudo abrir esa carpeta. Revisa que todavía existe."
        }
        "settings.history.ocr" => "Indexar texto para buscar",
        "settings.history.keep" => "Conservar capturas durante",
        "settings.history.keep.desc" => {
            "Las capturas más antiguas se eliminan permanentemente. Las destacadas se conservan."
        }
        "settings.history.limit" => "Límite de almacenamiento",
        "settings.history.limit.desc" => {
            "Al pasar el límite, las capturas más antiguas se eliminan permanentemente. Las destacadas se conservan."
        }
        "settings.history.pending" => {
            "Al aplicar esto, se eliminarán permanentemente {n} capturas ({size})."
        }
        "settings.history.pending_one" => {
            "Al aplicar esto, se eliminará permanentemente 1 captura ({size})."
        }
        "settings.history.pending_none" => "Ahora mismo no se eliminaría nada.",
        "settings.history.pending_checking" => "Contando capturas\u{2026}",
        "settings.history.pending_failed" => "No se pudo contar lo que se eliminaría.",
        "settings.history.apply" => "Aplicar",
        "settings.history.cancel" => "Cancelar",
        "settings.history.originals" => "Conservar originales editables",
        "settings.history.originals.desc" => {
            "Las anotaciones se pueden editar después. Lo que queda bajo un desenfoque, un pixelado o un recuadro opaco nunca se guarda."
        }
        "settings.privacy.card" => "Tus capturas nunca salen de tu PC",
        "settings.privacy.card.body_store" => {
            "Sin subidas, cuentas ni telemetría. El reconocimiento de texto funciona sin conexión en Windows. Stinkshot no hace ninguna solicitud de red. Microsoft Store se encarga de las actualizaciones."
        }
        "settings.privacy.card.body" => {
            "Sin subidas, cuentas ni telemetría. El reconocimiento de texto funciona sin conexión en Windows. La única solicitud de red es la búsqueda de actualizaciones de abajo, desactivada hasta que la actives."
        }
        "settings.privacy.updates" => "Buscar actualizaciones",
        "settings.privacy.updates.desc" => {
            "Descarga una vez al día la información firmada de la versión desde stinkshot.com. No envía nada más. Actualizar ahora solo descarga cuando haces clic."
        }
        "settings.privacy.clipboard_history" => "Agregar las copias al historial del portapapeles",
        "settings.privacy.clipboard_history.desc" => {
            "Permite que Win+V guarde lo que copia Stinkshot. Desactivado, las capturas no aparecen allí."
        }
        "settings.privacy.check_now" => "Comprobar ahora",
        "settings.privacy.manual_check" => "Comprobación manual",
        "settings.privacy.check_failed" => {
            "La última comprobación falló. Vuelve a intentarlo más tarde."
        }
        "settings.privacy.history.desc" => "Dónde se guardan las capturas y durante cuánto tiempo.",
        "settings.privacy.history.open" => "Configuración del historial",
        "settings.privacy.update_ready" => "Stinkshot {tag} está disponible",
        "settings.privacy.update_ready.desc" => {
            "Descarga el instalador y lo comprueba con la información firmada de la versión. Después se reinicia en la versión nueva."
        }
        "settings.privacy.update_page.desc" => {
            "No se instaló con el instalador. Consíguela en la página de descarga."
        }
        "settings.privacy.update_now" => "Actualizar ahora",
        "settings.privacy.download" => "Descargar",
        "settings.privacy.downloading" => "Descargando\u{2026} {pct}%",
        "settings.privacy.installing" => "Instalando\u{2026}",
        "settings.privacy.history" => "Historial",
        "settings.privacy.open_file" => "Abrir archivo",
        "settings.privacy.open_file_failed" => "No se pudo abrir el archivo",
        "settings.privacy.open_file_failed.hint" => {
            "Revisa que hay un editor de texto asignado a los archivos .toml."
        }
        "settings.about.license" => "Licencia",
        "settings.about.license.desc" => {
            "Software libre: úsalo, estúdialo, compártelo y modifícalo."
        }
        "settings.about.source" => "Código fuente",
        "settings.about.copied" => "Copiado",
        "settings.about.copy_link" => "Copiar enlace",
        "settings.about.link_failed" => "No se pudo abrir el enlace",
        "settings.about.link_failed.hint" => {
            "Windows no pudo abrir el navegador. Revisa que hay un navegador predeterminado."
        }
        "settings.about.copy_failed" => "No se pudo copiar",
        "settings.about.log" => "Registro",
        "settings.about.log.desc" => "Incluye stinkshot.log al reportar un problema.",
        "settings.about.open_log" => "Mostrar registro",
        "settings.about.log_failed" => "No se pudo mostrar el registro",
        "settings.about.report" => "Reportar un problema",
        "settings.about.report.desc" => "Abre la lista de problemas en el navegador.",
        "settings.about.report.open" => "Ver problemas",
        "settings.about.privacy" => "Privacidad",
        "settings.about.privacy.desc_store" => {
            "Sin subidas, cuentas ni telemetría. Stinkshot no hace ninguna solicitud de red. Microsoft Store se encarga de las actualizaciones."
        }
        "settings.about.privacy.desc" => {
            "Sin subidas, cuentas ni telemetría. La única solicitud de red es la búsqueda opcional de actualizaciones."
        }
        "settings.about.offline" => "Sin conexión por defecto",
        "settings.nav.help" => "Ayuda",
        "settings.about.tour" => "Recorrido",
        "settings.about.tour.desc" => "La breve introducción del primer inicio.",
        "settings.about.show_tour" => "Ver recorrido",
        "settings.help.hotkeys" => "Atajos globales",
        "settings.help.hotkeys.desc" => {
            "Funcionan en cualquier lugar mientras Stinkshot está abierto."
        }
        "settings.help.change_hotkeys" => "Cambiar atajos",
        "settings.help.editor" => "Atajos del editor",
        "settings.help.editor.desc" => "Presiona F1 o ? en el editor para ver esta lista.",
        "settings.help.tray" => "Icono de la bandeja",
        "settings.help.tray.click" => "Clic",
        "settings.help.tray.click.what" => "Capturar una región",
        "settings.help.tray.dbl" => "Doble clic",
        "settings.help.tray.dbl.what" => "Abrir el historial",
        "settings.help.tray.right" => "Clic derecho",
        "settings.help.tray.right.what" => "Todos los modos de captura y la configuración",
        "settings.help.features" => "Funciones",
        "settings.help.f.capture" => "Región, ventana, pantalla completa",
        "settings.help.f.capture.desc" => "Congela la pantalla y luego selecciona o haz clic.",
        "settings.help.f.scroll" => "Con desplazamiento",
        "settings.help.f.scroll.desc" => {
            "Desplaza una ventana y la captura como una sola imagen alta."
        }
        "settings.help.f.ocr" => "Copiar texto (OCR)",
        "settings.help.f.ocr.desc" => {
            "Lee el texto sin conexión con Windows. La búsqueda del historial también lo usa."
        }
        "settings.help.f.annotate" => "Anotar",
        "settings.help.f.annotate.desc" => {
            "Flechas, formas, texto, pasos, desenfoque y recuadros opacos."
        }
        "settings.help.f.sensitive" => "Buscar información sensible",
        "settings.help.f.sensitive.desc" => "Detecta correos, claves y números para ocultarlos.",
        "settings.help.f.beautify" => "Embellecer",
        "settings.help.f.beautify.desc" => "Enmarca una captura con margen y fondo.",
        "settings.help.f.pin" => "Fijar",
        "settings.help.f.pin.desc" => "Mantiene una captura encima de las demás ventanas.",
        "settings.help.f.record" => "Grabar MP4 o GIF",
        "settings.help.f.record.desc" => "Video codificado por hardware. Recorta antes de guardar.",
        "settings.help.f.history" => "Historial",
        "settings.help.f.history.desc" => {
            "Todas las capturas, con búsqueda. Ctrl+H desde la configuración."
        }
        "settings.help.files" => "Ubicación de los archivos",
        "settings.help.captures" => "Capturas",
        "settings.help.settings_file" => "Archivo de configuración",
        "settings.help.log" => "Registro",
        "settings.help.open" => "Abrir",
        "settings.help.show" => "Mostrar",
        "settings.help.open_failed" => "No se pudo abrir",
        "settings.help.open_failed.hint" => "Windows no pudo abrirlo. Revisa que la ruta existe.",
        _ => return None,
    })
}
