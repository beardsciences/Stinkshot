//! Spanish strings for history_ui.rs. Missing keys fall back to English.

pub fn get(key: &str) -> Option<&'static str> {
    Some(match key {
        "history.search.hint" => "Buscar capturas",
        "history.search.clear" => "Borrar búsqueda",
        "history.delete.title" => "¿Eliminar esta captura?",
        "history.date.filter" => "Filtro de fecha",
        "history.date.any" => "Cualquier fecha",
        "history.date.today" => "Hoy",
        "history.date.week" => "Últimos 7 días",
        "history.date.month" => "Últimos 30 días",
        "history.date.year" => "Últimos 365 días",
        "history.date.today_at" => "Hoy {time}",
        "history.date.yesterday_at" => "Ayer {time}",
        "history.app.unknown" => "Desconocida",
        "history.item.fallback" => "Captura",
        "history.empty.title" => "Aún no hay capturas",
        "history.empty.hint_tray" => {
            "Usa el botón de abajo o el icono de la bandeja. Las capturas se guardan aquí y puedes buscarlas por su texto."
        }
        "history.empty.hint_key" => {
            "Presiona {key} para capturar una región. Las capturas se guardan aquí y puedes buscarlas por su texto."
        }
        "history.empty.capture" => "Capturar una región",
        "history.nomatch.title" => "Nada coincide",
        "history.nomatch.body" => "Prueba otras palabras o borra los filtros.",
        "history.clear_filters" => "Borrar filtros",
        "history.retention.day" => "1 día",
        "history.retention.days" => "{n} días",
        "history.retention.days_and_size" => {
            "Se conservan {days} o {size}. Las destacadas se conservan siempre. En uso: {used}."
        }
        "history.retention.days_only" => {
            "Se conservan {days}. Las destacadas se conservan siempre. En uso: {used}."
        }
        "history.retention.size_only" => {
            "Se conservan hasta {size}. Las destacadas se conservan siempre. En uso: {used}."
        }
        "history.retention.everything" => "Se conserva todo. En uso: {used}.",
        "history.retention.tip" => "Cambiar cuánto se guarda el historial",
        "history.results.one" => "1 captura",
        "history.results.many" => "{n} capturas",
        "history.results.match_one" => {
            "1 captura contiene texto que coincide con \u{201c}{q}\u{201d}"
        }
        "history.results.match_many" => {
            "{n} capturas contienen texto que coincide con \u{201c}{q}\u{201d}"
        }
        "history.act.edit" => "Editar",
        "history.act.pin" => "Fijar en pantalla",
        "history.act.add_to_canvas" => "Agregar al lienzo",
        "history.act.copy_image" => "Copiar imagen",
        "history.act.copy_text" => "Copiar texto",
        "history.act.open" => "Abrir",
        "history.act.play" => "Reproducir",
        "history.act.reveal" => "Mostrar en la carpeta",
        "history.act.star" => "Destacar",
        "history.act.unstar" => "Quitar de destacadas",
        "history.act.delete" => "Eliminar\u{2026}",
        "history.act.no_ocr" => "Requiere un idioma con reconocimiento de texto en Windows",
        "history.act.indexing" => "Aún se está indexando el texto",
        "history.act.no_text" => "No se encontró texto",
        "history.act.images_only" => "Solo imágenes",
        "history.error.load" => "No se pudo leer el índice del historial. {why}",
        "history.error.load_fix" => {
            "Reinicia Stinkshot. Si sigue pasando, elige otra carpeta de historial en Configuración."
        }
        "history.nav.all" => "Todas las capturas",
        "history.nav.starred" => "Destacadas",
        "history.nav.recordings" => "Grabaciones",
        "history.nav.source_app" => "Aplicación de origen",
        "history.nav.settings" => "Abrir configuración",
        "history.nav.settings_tip" => "Abrir configuración ({key})",
        "history.open_folder" => "Abrir carpeta",
        "history.no_ocr" => {
            "La búsqueda solo abarca títulos de ventana y nombres de aplicación. {help}"
        }
        "history.folder_error.title" => "No se pudo abrir la carpeta",
        "history.folder_error.fix" => {
            "Windows no pudo abrir la carpeta del historial. Revísala en Configuración > Historial."
        }
        "history.delete.body" => {
            "El archivo va a la Papelera de reciclaje. Enter elimina, Esc lo conserva."
        }
        "history.delete.body_starred" => {
            "Está destacada. El archivo va a la Papelera de reciclaje. Enter elimina, Esc lo conserva."
        }
        "history.delete.cancel" => "Cancelar",
        "history.delete.confirm" => "Eliminar",
        "history.thumb.unavailable" => "Vista previa no disponible",
        "history.mode.window" => "Ventana",
        "history.mode.fullscreen" => "Pantalla completa",
        "history.mode.scrolling" => "Con desplazamiento",
        "history.mode.recording" => "Grabación",
        "history.mode.text" => "Texto seleccionado",
        _ => return None,
    })
}
