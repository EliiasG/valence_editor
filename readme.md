# valence_editor
An editor that saved structures with [valence_vstruc](https://github.com/EliiasG/valence_vstruc).  
To install this, clone the repo and run `cargo install --path .`  
You will now be able to run `valence_editor` in whatever directory you want to work in and connect to `localhost` in minecraft to use it.  
### Within minecraft you can run the following commands:

| Command | Function |
| - | -|
| `/save` or `/s` | Saves the current structure. Will save to the previousley saved/loaded path if no path is given
| `/load` or `/l` | Loads a structure from the given path
| `/new` | Deletes the currently placed blocks
| `/origin` or `/o`| Moves the origin in the desired direction, or to the player with `/origin here`
| `/path` or `/p` | Get the local path of the current structure

*This is not officially associated with valence