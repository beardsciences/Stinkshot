# Microsoft Store package

The Store build is the same exe as the installer build, packaged as MSIX. The Store signs it,
installs it and updates it. Store installs show no SmartScreen warning.

## What changes in the Store build

The app checks `stinkshot_platform::system::is_packaged()` at run time:

| Feature | Installer build | Store build |
| --- | --- | --- |
| Updates | Optional update check, Update Now | The Store. Update check, Update Now and `stinkshot update` are hidden (Store policy 10.2.2: no downloading code to run). |
| Start with Windows | HKCU Run value | The package's startup task (`StinkshotStartup`). A startup launch counts as `--tray`. |
| `stinkshot` command | Installer adds the exe | App execution alias `stinkshot.exe` |
| Settings and History | `%APPDATA%\Stinkshot`, `Pictures\Stinkshot` | Same places (write virtualization is off) |

## Build

```
powershell -ExecutionPolicy Bypass -File scripts\store-package.ps1 -IdentityName <Name> -Publisher <CN=...> -PublisherDisplayName <name>
```

The three values come from Partner Center > the app > Product identity, after the name is
reserved. Output: `dist\store\<version>\Stinkshot-<version>-x64.msix`.

Local test (separate identity, settings, History and single-instance lock, so it runs beside
an installed copy):

```
powershell -ExecutionPolicy Bypass -File scripts\store-package.ps1 -Test
Add-AppxPackage -Register dist\store\<version>-test\layout\AppxManifest.xml
```

Registering an unsigned layout needs Developer Mode (Settings > System > For developers).
Remove it with `Get-AppxPackage Stinkshot.LocalTest | Remove-AppxPackage`.

## Product identity

Reserved Nov 23 2026. Store ID 9MX55GVGVBKL.

```
powershell -ExecutionPolicy Bypass -File scripts\store-package.ps1 -IdentityName ChristianMcCullough.Stinkshot -Publisher "CN=072663B3-B5AE-41B8-9003-1FEEC40A88D6" -PublisherDisplayName "Christian McCullough"
```

Package family name: `ChristianMcCullough.Stinkshot_eejxjq58zvpf0`.

## Partner Center, once

1. Create the developer account at storedeveloper.microsoft.com (individual, free; ID and selfie
   check).
2. Reserve the name "Stinkshot".
3. Copy Package/Identity/Name, Package/Identity/Publisher and Package/Properties/
   PublisherDisplayName from Product identity into the build command above.

## Each submission

1. Build with the identity values. Upload the .msix under Packages.
2. Store listing: text below, screenshots from stinkshot.com's media (1920x1080 or larger).
3. Privacy policy URL: https://stinkshot.com/privacy/
4. Age rating questionnaire: no user-generated content shared, no data collected.
5. Paste the certification notes below into "Notes for certification".

## Certification notes (restricted capabilities)

Stinkshot is an open-source (GPL-3.0) screenshot and screen recording tool for Windows.
Source: https://code.mccullough.dev/christian/stinkshot

- runFullTrust: it is a Win32 desktop app (Rust, egui).
- unvirtualizedResources, with RegistryWriteVirtualization and FileSystemWriteVirtualization
  disabled:
  - Settings are a plain TOML file in %APPDATA%\Stinkshot that users can open and edit from
    Settings > Privacy > Open File. With virtualization, the path shown and the file other
    editors open would not be the one the app uses.
  - "Take Over PrintScreen" (Settings > Hotkeys) sets the user's Windows option
    HKCU\Control Panel\Keyboard\PrintScreenKeyForSnippingEnabled to 0 so PrtSc reaches the
    app, and restores the previous value when turned off or on quit. With virtualization the
    change would not reach Windows.
- Keyboard hook: "Replace Snipping Tool" (off by default) uses a low-level keyboard hook that
  only acts on Win+Shift+S, Win+Shift+R and PrtSc and passes every other key through. It
  records nothing.
- microphone, webcam: optional sources for screen recordings, off by default.
- Network: none. The update check of the installer build is disabled in this build.

## Store listing text

Short description:

> Private screenshots and screen recording for Windows. Nothing leaves your PC.

Description:

> Stinkshot captures a region, a window, the full screen or a whole scrolling page, and lets
> you annotate it on the spot: arrows, shapes, text, numbered steps, highlights, blur and
> redaction boxes that destroy the pixels under them.
>
> Copy Text reads text from anything on screen, offline, using Windows' own text recognition.
> History keeps every capture, searchable by the text in it. Record the screen as MP4 or GIF,
> with system audio, microphone and a webcam bubble, then trim before saving.
>
> No uploads, accounts or telemetry. Stinkshot makes no network requests.
>
> Free and open source (GPL-3.0).

Features (one per line in Partner Center):

- Region, window, full screen and scrolling capture
- Annotate in place, with redaction that destroys pixels
- Copy text from the screen, offline
- Searchable History of every capture
- MP4 and GIF recording with trim
- English and Spanish

Screenshot captions, in order (`dist\store\listing\`):

1. Annotate in place: arrows, steps, labels
2. Every capture in History
3. Search captures by the text in them
4. Nothing leaves your PC
5. Record MP4 or GIF
6. Light and dark themes
7. Filter History by app
8. Free and open source

## Store listing text, Spanish (es)

Latin American Spanish, using the app's own terms (Historial, Copiar texto, Captura con
desplazamiento). Screenshots: `dist\store\listing-es\`.

Descripción breve:

> Capturas y grabaciones de pantalla privadas para Windows. Nada sale de tu PC.

Descripción:

> Stinkshot captura una región, una ventana, la pantalla completa o una página entera con
> desplazamiento, y te deja anotarla al instante: flechas, formas, texto, pasos numerados,
> resaltados, desenfoque y recuadros opacos que destruyen los píxeles que cubren.
>
> Copiar texto lee el texto de todo lo que aparece en pantalla, sin conexión, con el
> reconocimiento de texto del propio Windows. El Historial guarda cada captura y permite buscarla por el texto que
> contiene. Graba la pantalla en MP4 o GIF, con audio del sistema, micrófono y una burbuja de
> cámara web, y recorta antes de guardar.
>
> Sin subidas, cuentas ni telemetría. Stinkshot no realiza ninguna solicitud de red.
>
> Gratis y de código abierto (GPL-3.0).

Características (una por línea en Partner Center):

- Captura de región, ventana, pantalla completa y con desplazamiento
- Anota al instante, con recuadros opacos que destruyen los píxeles
- Copia texto de la pantalla, sin conexión
- Historial de todas tus capturas, con búsqueda
- Grabación en MP4 y GIF con recorte
- En inglés y español

Títulos de las capturas, en orden:

1. Anota al instante: flechas, pasos y texto
2. Todas tus capturas en el Historial
3. Busca capturas por el texto que contienen
4. Nada sale de tu PC
5. Graba en MP4 o GIF
6. Temas claro y oscuro
7. Filtra el Historial por aplicación
8. Gratis y de código abierto
