<p align="center">
  <img src="https://github.com/user-attachments/assets/41dd6587-9f3c-45cd-b6b4-e144dc4338ac" style="background-color: black" alt="godot-spacetimedb_128" width="128">
</p>

# SpacetimeDB + Godot + Rust
This is a demo for how to use SpacetimeDB in Godot with Rust, based on unity demo [blackholio](https://github.com/clockworklabs/SpacetimeDB/tree/master/demo/Blackholio)  
It looks good and works well.

## Problems
* Godot axis-y is opposite to Unity, if you connect unity and godot to the same db, you will find the position of the object is different.
* Using Rust syntax in Godot is "weird", like if you have a `process`, then you can't borrow it in other places, so I write some eccentric code to make it work.