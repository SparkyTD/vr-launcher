# vr-launcher

___
### 🚧 This project is still work in progress. Expect bugs and unstable features. 🚧
___

A web-based launcher for your PCVR games, using WiVRn or an Envision profile as the backend. 

This project aims to simplify the multistep process of launching PCVR games on standalone HMDs like the Quest 2/3, by providing a simple web-based interface that can be opened from the HMD's built-in browser.

It does this by automatically provisioning the VR environment, launches WiVRn and wlx-overlay-x, as well as the selected game, just by a press of a button on the web interface. As long as the launcher server is running in the background, you can always easily launch any compatible PCVR game straight from your standalone VR headset.

![Screenshot of the web app](media/screenshot1.jpg)

## Features
- Launch any supported VR game. For now, each game must be manually entered into the database.
- Adjust system audio settings (PipeWire only!)
- Monitor HMD stats like battery level and charging state
- Launch games using WiVRn as a backend
- Launch games using a WiVRn-based Envision profile as the backend
- Built-in support for wlx-overlay-s
- Monitor playtime, close active game