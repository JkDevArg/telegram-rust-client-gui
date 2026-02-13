# Cliente de Telegram en Rust

Este proyecto es un cliente de Telegram con interfaz gráfica (GUI) escrito en Rust. Utiliza las librerías `grammers` para la comunicación con la API de Telegram y `eframe` (egui) para la interfaz de usuario.

## Características

-   **Configuración Personalizable**: Permite ingresar tu propio `API ID` y `API Hash` al iniciar.
-   **Inicio de Sesión Gráfico**: Soporte completo para el flujo de autenticación (Número de teléfono, Código de verificación y Contraseña de doble factor/2FA).
-   **Lista de Chats**: Visualiza tus chats recientes.
-   **Historial de Mensajes**: Lee los últimos 50 mensajes de cualquier chat.
-   **Envío de Mensajes**: Envía mensajes de texto directamente desde la interfaz.
-   **Cierre de Sesión**: Botón integrado para cerrar sesión y limpiar las credenciales locales.

## Requisitos Técnicos

-   **Lenguaje**: Rust (Edición 2021)
-   **GUI**: `eframe` (egui)
-   **Async Runtime**: `tokio`
-   **Cliente Telegram**: `grammers`

## Instrucciones de Uso

Para ejecutar la aplicación, asegúrate de tener Rust instalado y corre el siguiente comando en la terminal dentro del directorio del proyecto:

```sh
cargo run
```

### Primeros Pasos:
1.  Al abrir la app, confirma o edita las credenciales de la API (API ID y Hash) y guarda la configuración.
2.  Ingresa tu número de teléfono (formato internacional, ej: `+51999999999`).
3.  Ingresa el código que recibirás en tu app oficial de Telegram.
4.  Si tienes verificación en dos pasos (2FA), ingresa tu contraseña.
5.  ¡Listo! Podrás ver tus chats y enviar mensajes.

### Versión

Versión 0.1.0 del cliente de Telegram en Rust.
Puede que se agregue mas cosas.

