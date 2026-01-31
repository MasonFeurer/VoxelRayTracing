# Voxel Ray Tracing
Another Voxel Ray Tracing Engine, one with a creative name.
This project has the goal of creating an easy-to-use, "realistic" looking, smaller voxel engine.

![Screenshot](./pictures/ss-2025-11-17_17-56-06.png)
![Screenshot](./pictures/ss-2025-11-21_16-02-29.png)

# World
The world is organized into a grid of chunks 32x32x32 voxels in volume.
Each chunk stores it's voxels as an SVO (Sparse Voxel Octree) to optimize storage and rendering.

# Rendering
The rendering does not use any triangle mesh's. Intead, the raw SVO data is uploaded to the gpu, 
then the gpu casts a bunch of Ray's from the camera position into the world, using a partly-hand-crafted
algorithm for stepping through an SVO world efficiently. 

This rendering technique also makes it trivial to implement real-time light path-tracing to greatly improve the
visuals.

# How to Play
Download the git project using this bash command:
```sh
git clone "https://github.com/MasonFeurer/VoxelRayTracing.git"
```
Then, use `cargo` to build and run the app from source:
```sh
cd VoxelRayTracing/clientdesktop
cargo r -r
```