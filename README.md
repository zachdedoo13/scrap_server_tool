example settings.json
  {
  "steam_game_id": "387990", # this is found in steam_libary/right_click_scrap/propertys/updates/app_id

  "save_location": "C:\\Users\\zacha\\AppData\\Roaming\\Axolot Games\\Scrap Mechanic\\User\\User_76561198961850617\\Save\\Survival\\FirstSuvivel.db",
        # this path should be similar just use a different save, eg ServerWorld.db

  "backup_output_path": "C:\\Users\\zacha\\RustroverProjects\\scrap_server_tool\\test_out", # this is the folder where backups are saved, happens every 3 minutes

  "auto_save_interval_sec": 180, # Self explanatory

  "set_args": false # set to false by default, instructions printed to terminal when program is ran
  }

without comments 
  {
    "steam_game_id": "387990",
    "save_location": "C:\\Users\\zacha\\AppData\\Roaming\\Axolot Games\\Scrap Mechanic\\User\\User_76561198961850617\\Save\\Survival\\FirstSuvivel.db",
    "backup_output_path": "C:\\Users\\zacha\\RustroverProjects\\scrap_server_tool\\example\\out",
    "auto_save_interval_sec": 5,
    "set_args": true
  }

a prebuilt exacutable is in the example folder 
