## Variables that change between games

### ✅ AMD_VK_PIPELINE_CACHE_PATH
- Folder path to `$STEAM_HOME/steamapps/shadercache/$APP_ID/AMDv1`
- Example: `/home/sparky/.local/share/Steam/steamapps/shadercache/3846119739/AMDv1`
- Note: `$APP_ID` seems to just be a random 10-digit code for non-steam games

### ✅ MESA_GLSL_CACHE_DIR, MESA_SHADER_CACHE_DIR, STEAM_COMPAT_SHADER_PATH, STEAM_COMPAT_TRANSCODED_MEDIA_PATH
- Folder path to `$STEAM_HOME/steamapps/shadercache/$APP_ID`
- Example: `/home/sparky/.local/share/Steam/steamapps/shadercache/3846119739`

### EnableConfiguratorSupport
- Numeric bitfield
- Bit 1: PS (mask & 1)
- Bit 2: Xbox (mask & 2)
- Bit 3: Generic (mask & 4)
- Bit 4: Switch (mask & 8)
- Example: `0` for external games; `65526` for VRChat

### SDL_GAMECONTROLLER_IGNORE_DEVICES, SteamGenericControllers
- List of comma separated, hex-formatted `0xVID/0xPID` pairs, referring to hardware controller devices
- Example: `0x28de/0x1002,0x28de/0x1003,0x28de/0x1071`
- Note that `SDL_GAMECONTROLLER_IGNORE_DEVICES` and `SteamGenericControllers` are different lists using the same format

### ✅ STEAM_COMPAT_APP_ID, SteamAppId
- `$APP_ID` for Steam games, `0` for external games

### ✅ SteamGameId, SteamOverlayGameId
- `$APP_ID` for Steam games, random 20-digit code for external games

### ✅ STEAM_COMPAT_DATA_PATH
- Wine prefix path
- Folder path to `$STEAM_HOME/steamapps/compatdata/$APP_ID`
- Example: `/home/sparky/.local/share/Steam/steamapps/compatdata/3846119739`

### ✅ STEAM_COMPAT_MEDIA_PATH
- Folder path to `$STEAM_HOME/steamapps/shadercache/$APP_ID/fozmediav1`
- Example: `/home/sparky/.local/share/Steam/steamapps/shadercache/3846119739/fozmediav1`

### ✅ __GL_SHADER_DISK_CACHE_PATH
- Folder path to `$STEAM_HOME/steamapps/shadercache/$APP_ID/fozmediav1`
- Example: `/home/sparky/.local/share/Steam/steamapps/shadercache/3846119739/fozmediav1`

### ✅ STEAM_COMPAT_MOUNTS
- Colon separated list of paths, including `$STEAM_HOME/steamapps/common/SteamLinuxRuntime_sniper`
- For Steam games, it also includes `$STEAM_HOME/steamapps/common/Steamworks Shared`
- Example: `/home/sparky/.local/share/Steam/steamapps/common/SteamLinuxRuntime_sniper`

### ✅ STEAM_FOSSILIZE_DUMP_PATH
- Folder path to `$STEAM_HOME/steamapps/shadercache/$APP_ID/fozpipelinesv6/steamapprun_pipeline_cache`
- Example: `/home/sparky/.local/share/Steam/steamapps/shadercache/3846119739/fozpipelinesv6/steamapprun_pipeline_cache`

### ✅ STEAM_COMPAT_TOOL_PATHS
- Colon-separated list of paths including the Proton directory and SteamLinuxRuntime_sniper
- Format: `$PROTON_DIRECTORY:$STEAM_HOME/steamapps/common/SteamLinuxRuntime_sniper`
- Example: `/home/sparky/.local/share/Steam/compatibilitytools.d/GE-Proton9-22-rtsp17:/home/sparky/.local/share/Steam/steamapps/common/SteamLinuxRuntime_sniper`