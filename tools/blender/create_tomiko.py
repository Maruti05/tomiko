"""
Generate the original Tomiko character asset.

Run with:
  blender --background --python tools/blender/create_tomiko.py

Outputs:
  assets/characters/tomiko/character.glb
  assets/characters/tomiko/thumbnail.png
  assets/characters/tomiko/metadata.json

This is a procedural, stylized cat built from rounded forms, with a real
armature, 15 viseme shape keys, and 11 animation clips. It is intentionally
original and not based on any existing character IP.
"""

import bpy
import json
import math
import os
from mathutils import Vector, Euler

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT_DIR = os.path.join(ROOT, "assets", "characters", "tomiko")
GLB_PATH = os.path.join(OUT_DIR, "character.glb")
THUMB_PATH = os.path.join(OUT_DIR, "thumbnail.png")
META_PATH = os.path.join(OUT_DIR, "metadata.json")

os.makedirs(OUT_DIR, exist_ok=True)


def clear_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for datablocks in (
        bpy.data.meshes,
        bpy.data.armatures,
        bpy.data.materials,
        bpy.data.actions,
        bpy.data.cameras,
        bpy.data.lights,
    ):
        for block in list(datablocks):
            if block.users == 0:
                datablocks.remove(block)


def make_material(name, color, roughness=0.6, metallic=0.0):
    mat = bpy.data.materials.get(name)
    if mat is None:
        mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    if bsdf:
        bsdf.inputs["Base Color"].default_value = (*color, 1.0)
        bsdf.inputs["Roughness"].default_value = roughness
        bsdf.inputs["Metallic"].default_value = metallic
    return mat


def add_sphere(name, location, scale, material, segments=32, rings=20):
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=segments,
        ring_count=rings,
        radius=1.0,
        location=location,
    )
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.data.materials.append(material)
    bpy.ops.object.shade_smooth()
    return obj


def add_cone(name, location, radius1, radius2, depth, rotation, material):
    bpy.ops.mesh.primitive_cone_add(
        vertices=32,
        radius1=radius1,
        radius2=radius2,
        depth=depth,
        location=location,
        rotation=rotation,
    )
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(material)
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    bpy.ops.object.shade_smooth()
    return obj


def add_capsule(name, p1, p2, radius, material):
    p1 = Vector(p1)
    p2 = Vector(p2)
    direction = p2 - p1
    length = direction.length
    midpoint = (p1 + p2) * 0.5
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=24,
        radius=radius,
        depth=length,
        location=midpoint,
    )
    obj = bpy.context.object
    obj.name = name
    obj.rotation_mode = "QUATERNION"
    obj.rotation_quaternion = direction.to_track_quat("Z", "Y")
    obj.data.materials.append(material)
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    bpy.ops.object.shade_smooth()

    # Rounded ends.
    add_sphere(name + "_a", p1, (radius, radius, radius), material, segments=20, rings=12)
    add_sphere(name + "_b", p2, (radius, radius, radius), material, segments=20, rings=12)
    return obj


def build_character():
    fur = make_material("Fur_Orange", (0.72, 0.26, 0.08), roughness=0.72)
    fur_light = make_material("Fur_Cream", (0.93, 0.78, 0.58), roughness=0.8)
    inner_ear = make_material("Accent_Pink", (0.90, 0.42, 0.46), roughness=0.62)
    eye_white = make_material("Eye_White", (0.98, 0.97, 0.94), roughness=0.25)
    eye_black = make_material("Eye_Dark", (0.018, 0.014, 0.012), roughness=0.2)
    nose_pink = inner_ear
    mouth_dark = eye_black

    parts = []

    # Body and belly patch.
    parts.append(add_sphere("Body", (0, 0, 0.78), (0.42, 0.30, 0.55), fur))
    parts.append(add_sphere("Belly", (0, -0.18, 0.70), (0.26, 0.15, 0.32), fur_light))

    # Head. The muzzle and eyes are intentionally separated so the eye whites
    # and pupils do not intersect the cream muzzle.
    parts.append(add_sphere("Head", (0, 0, 1.36), (0.40, 0.38, 0.36), fur))
    parts.append(add_sphere("Muzzle", (0, -0.385, 1.225), (0.15, 0.105, 0.085), fur_light))

    # Ears.
    parts.append(
        add_cone("Ear_L", (0.23, 0.0, 1.66), 0.15, 0.0, 0.30, (0, math.radians(14), 0), fur)
    )
    parts.append(
        add_cone("Ear_R", (-0.23, 0.0, 1.66), 0.15, 0.0, 0.30, (0, math.radians(-14), 0), fur)
    )
    parts.append(
        add_cone(
            "Ear_L_Inner",
            (0.23, -0.035, 1.66),
            0.085,
            0.0,
            0.22,
            (0, math.radians(14), 0),
            inner_ear,
        )
    )
    parts.append(
        add_cone(
            "Ear_R_Inner",
            (-0.23, -0.035, 1.66),
            0.085,
            0.0,
            0.22,
            (0, math.radians(-14), 0),
            inner_ear,
        )
    )

    # Eyes, pupils, highlights.
    for side, x in (("L", 0.17), ("R", -0.17)):
        parts.append(add_sphere(f"Eye_{side}", (x, -0.395, 1.46), (0.090, 0.070, 0.100), eye_white))
        parts.append(add_sphere(f"Pupil_{side}", (x, -0.460, 1.46), (0.047, 0.028, 0.057), eye_black))
        parts.append(
            add_sphere(
                f"Highlight_{side}",
                (x + 0.015, -0.490, 1.48),
                (0.012, 0.006, 0.014),
                eye_white,
                segments=16,
                rings=10,
            )
        )

    # Nose and mouth patch.
    parts.append(add_sphere("Nose", (0, -0.505, 1.27), (0.035, 0.022, 0.022), nose_pink))
    parts.append(add_sphere("Mouth", (0, -0.505, 1.195), (0.085, 0.012, 0.020), mouth_dark))

    # Arms.
    for side, sx in (("L", 1.0), ("R", -1.0)):
        parts.append(
            add_capsule(
                f"Arm_{side}",
                (0.36 * sx, 0.0, 0.96),
                (0.56 * sx, -0.04, 0.56),
                0.075,
                fur,
            )
        )
        parts.append(add_sphere(f"Paw_{side}", (0.56 * sx, -0.055, 0.54), (0.10, 0.09, 0.09), fur_light))

    # Legs and feet.
    for side, sx in (("L", 1.0), ("R", -1.0)):
        parts.append(
            add_capsule(
                f"Leg_{side}",
                (0.18 * sx, 0.0, 0.48),
                (0.22 * sx, -0.02, 0.11),
                0.105,
                fur,
            )
        )
        parts.append(add_sphere(f"Foot_{side}", (0.22 * sx, -0.10, 0.065), (0.14, 0.20, 0.07), fur_light))

    # Tail: a chain of rounded segments.
    tail_points = [
        (0.0, 0.27, 0.55),
        (0.05, 0.40, 0.63),
        (0.10, 0.50, 0.78),
        (0.12, 0.53, 0.94),
        (0.10, 0.48, 1.08),
    ]
    for i in range(len(tail_points) - 1):
        radius = 0.065 - i * 0.008
        parts.append(add_capsule(f"Tail_{i}", tail_points[i], tail_points[i + 1], radius, fur))
    parts.append(add_sphere("TailTip", tail_points[-1], (0.055, 0.055, 0.055), fur_light))

    # Join everything into one mesh, including capsule end caps created by
    # add_capsule(). Keep exactly one skinned mesh for the character.
    bpy.ops.object.select_all(action="DESELECT")
    mesh_objects = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    for obj in mesh_objects:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = mesh_objects[0]
    bpy.ops.object.join()
    character = bpy.context.object
    character.name = "TomikoMesh"

    # Put origin at world origin so shape-key selection uses world coordinates.
    bpy.context.scene.cursor.location = (0, 0, 0)
    bpy.ops.object.origin_set(type="ORIGIN_CURSOR")

    # Weld duplicate/overlapping vertices lightly? Keep separate shells; this is
    # a stylized character and avoids collapsing thin features.
    return character


def add_shape_keys(character):
    character.shape_key_add(name="Basis", from_mix=False)

    mouth_center = Vector((0.0, -0.505, 1.195))

    def mouth_vertices():
        result = []
        for i, vertex in enumerate(character.data.vertices):
            co = vertex.co
            if abs(co.x) < 0.20 and co.y < -0.47 and 1.08 < co.z < 1.29:
                result.append(i)
        return result

    def eye_vertices():
        result = []
        for i, vertex in enumerate(character.data.vertices):
            co = vertex.co
            for ex in (0.17, -0.17):
                center = Vector((ex, -0.42, 1.46))
                if (co - center).length < 0.15:
                    result.append(i)
                    break
        return result

    mouth_ids = mouth_vertices()
    eye_ids = eye_vertices()

    def make_viseme(name, transform):
        key = character.shape_key_add(name=name, from_mix=False)
        key.slider_min = 0.0
        key.slider_max = 1.0
        key.value = 0.0
        for i in mouth_ids:
            original = character.data.vertices[i].co.copy()
            key.data[i].co = transform(original, mouth_center)
        return key

    make_viseme("viseme_AA", lambda co, c: Vector((co.x, co.y - 0.012, c.z + (co.z - c.z) * 1.9)))
    make_viseme("viseme_E", lambda co, c: Vector((co.x * 1.35, co.y - 0.004, c.z + (co.z - c.z) * 0.65)))
    make_viseme("viseme_I", lambda co, c: Vector((co.x * 1.18, co.y - 0.002, c.z + (co.z - c.z) * 0.50)))
    make_viseme("viseme_O", lambda co, c: Vector((co.x * 0.62, co.y - 0.020, c.z + (co.z - c.z) * 1.65)))
    make_viseme("viseme_U", lambda co, c: Vector((co.x * 0.45, co.y - 0.028, c.z + (co.z - c.z) * 1.15)))
    make_viseme("viseme_M", lambda co, c: Vector((co.x * 1.04, co.y + 0.008, c.z + (co.z - c.z) * 0.18)))
    make_viseme("viseme_F", lambda co, c: Vector((co.x * 1.08, co.y + 0.012, c.z + (co.z - c.z) * 0.32)))
    make_viseme("viseme_L", lambda co, c: Vector((co.x * 0.92, co.y - 0.010, c.z + (co.z - c.z) * 0.75 + 0.012)))
    make_viseme("viseme_R", lambda co, c: Vector((co.x * 0.80, co.y - 0.006, c.z + (co.z - c.z) * 0.72)))
    make_viseme("viseme_S", lambda co, c: Vector((co.x * 1.22, co.y + 0.010, c.z + (co.z - c.z) * 0.16)))
    make_viseme("viseme_SH", lambda co, c: Vector((co.x * 0.82, co.y - 0.026, c.z + (co.z - c.z) * 0.55)))
    make_viseme("viseme_TH", lambda co, c: Vector((co.x * 0.95, co.y - 0.022, c.z + (co.z - c.z) * 0.45 - 0.010)))
    make_viseme("viseme_W", lambda co, c: Vector((co.x * 0.50, co.y - 0.024, c.z + (co.z - c.z) * 1.05)))
    make_viseme("viseme_REST", lambda co, c: co.copy())

    blink = character.shape_key_add(name="viseme_BLINK", from_mix=False)
    blink.slider_min = 0.0
    blink.slider_max = 1.0
    blink.value = 0.0
    for i in eye_ids:
        co = character.data.vertices[i].co.copy()
        # Collapse the eye vertically toward its center line.
        co.z = 1.46 + (co.z - 1.46) * 0.06
        blink.data[i].co = co

    print(f"Shape keys: {len(character.data.shape_keys.key_blocks)}")
    print(f"  mouth vertices: {len(mouth_ids)}")
    print(f"  eye vertices: {len(eye_ids)}")


def build_armature(character):
    arm_data = bpy.data.armatures.new("TomikoArmature")
    arm_obj = bpy.data.objects.new("TomikoArmature", arm_data)
    bpy.context.collection.objects.link(arm_obj)
    bpy.context.view_layer.objects.active = arm_obj
    bpy.ops.object.mode_set(mode="EDIT")

    bones = {}

    def bone(name, head, tail, parent=None):
        b = arm_data.edit_bones.new(name)
        b.head = head
        b.tail = tail
        b.use_connect = False
        if parent:
            b.parent = bones[parent]
        bones[name] = b
        return b

    bone("root", (0, 0, 0), (0, 0, 0.20))
    bone("hips", (0, 0, 0.43), (0, 0, 0.66), "root")
    bone("spine", (0, 0, 0.66), (0, 0, 0.92), "hips")
    bone("chest", (0, 0, 0.92), (0, 0, 1.12), "spine")
    bone("neck", (0, 0, 1.12), (0, 0, 1.25), "chest")
    bone("head", (0, 0, 1.25), (0, 0, 1.58), "neck")
    bone("ear.L", (0.18, 0, 1.58), (0.29, 0, 1.82), "head")
    bone("ear.R", (-0.18, 0, 1.58), (-0.29, 0, 1.82), "head")

    bone("shoulder.L", (0.12, 0, 1.02), (0.36, 0, 0.96), "chest")
    bone("upper_arm.L", (0.36, 0, 0.96), (0.51, -0.02, 0.74), "shoulder.L")
    bone("forearm.L", (0.51, -0.02, 0.74), (0.55, -0.04, 0.57), "upper_arm.L")
    bone("hand.L", (0.55, -0.04, 0.57), (0.56, -0.06, 0.47), "forearm.L")

    bone("shoulder.R", (-0.12, 0, 1.02), (-0.36, 0, 0.96), "chest")
    bone("upper_arm.R", (-0.36, 0, 0.96), (-0.51, -0.02, 0.74), "shoulder.R")
    bone("forearm.R", (-0.51, -0.02, 0.74), (-0.55, -0.04, 0.57), "upper_arm.R")
    bone("hand.R", (-0.55, -0.04, 0.57), (-0.56, -0.06, 0.47), "forearm.R")

    bone("thigh.L", (0.17, 0, 0.48), (0.20, -0.02, 0.27), "hips")
    bone("shin.L", (0.20, -0.02, 0.27), (0.22, -0.04, 0.10), "thigh.L")
    bone("foot.L", (0.22, -0.04, 0.10), (0.22, -0.18, 0.06), "shin.L")

    bone("thigh.R", (-0.17, 0, 0.48), (-0.20, -0.02, 0.27), "hips")
    bone("shin.R", (-0.20, -0.02, 0.27), (-0.22, -0.04, 0.10), "thigh.R")
    bone("foot.R", (-0.22, -0.04, 0.10), (-0.22, -0.18, 0.06), "shin.R")

    bone("tail.01", (0, 0.26, 0.55), (0, 0.40, 0.64), "hips")
    bone("tail.02", (0, 0.40, 0.64), (0.08, 0.52, 0.80), "tail.01")
    bone("tail.03", (0.08, 0.52, 0.80), (0.12, 0.53, 0.98), "tail.02")
    bone("tail.04", (0.12, 0.53, 0.98), (0.10, 0.48, 1.10), "tail.03")

    bpy.ops.object.mode_set(mode="OBJECT")

    # Parent with automatic weights.
    bpy.ops.object.select_all(action="DESELECT")
    character.select_set(True)
    arm_obj.select_set(True)
    bpy.context.view_layer.objects.active = arm_obj
    bpy.ops.object.parent_set(type="ARMATURE_AUTO")

    print(f"Bones: {len(arm_data.bones)}")
    return arm_obj


def reset_pose(arm_obj):
    for pb in arm_obj.pose.bones:
        pb.rotation_mode = "XYZ"
        pb.rotation_euler = (0.0, 0.0, 0.0)
        pb.location = (0.0, 0.0, 0.0)
        pb.scale = (1.0, 1.0, 1.0)


def key_all(arm_obj, frame):
    for pb in arm_obj.pose.bones:
        pb.keyframe_insert(data_path="rotation_euler", frame=frame)
        pb.keyframe_insert(data_path="location", frame=frame)


def key_bone(arm_obj, name, frame, rot=None, loc=None, scale=None):
    pb = arm_obj.pose.bones[name]
    pb.rotation_mode = "XYZ"
    if rot is not None:
        pb.rotation_euler = rot
        pb.keyframe_insert(data_path="rotation_euler", frame=frame)
    if loc is not None:
        pb.location = loc
        pb.keyframe_insert(data_path="location", frame=frame)
    if scale is not None:
        pb.scale = scale
        pb.keyframe_insert(data_path="scale", frame=frame)


def create_action(arm_obj, name, end_frame, poses):
    arm_obj.animation_data_create()
    action = bpy.data.actions.new(name)
    action.use_fake_user = True
    arm_obj.animation_data.action = action
    reset_pose(arm_obj)
    key_all(arm_obj, 1)
    key_all(arm_obj, end_frame)
    for frame, bone_name, rot, loc in poses:
        key_bone(arm_obj, bone_name, frame, rot=rot, loc=loc)
    arm_obj.animation_data.action = None
    reset_pose(arm_obj)
    print(f"Action {name}: frames 1..{end_frame}")


def build_actions(arm_obj):
    # Bone local Y points along the bone; X/Z give forward/side tilt.
    create_action(
        arm_obj,
        "idle",
        60,
        [
            (15, "chest", (0.02, 0.0, 0.015), None),
            (30, "head", (0.03, 0.0, -0.02), None),
            (45, "chest", (-0.015, 0.0, -0.015), None),
            (30, "tail.02", (0.0, 0.0, 0.18), None),
            (60, "tail.02", (0.0, 0.0, -0.18), None),
        ],
    )
    create_action(
        arm_obj,
        "listening",
        45,
        [
            (12, "head", (0.0, 0.0, 0.18), None),
            (30, "head", (-0.06, 0.0, 0.10), None),
            (20, "ear.L", (0.0, 0.0, 0.16), None),
            (20, "ear.R", (0.0, 0.0, -0.16), None),
        ],
    )
    create_action(
        arm_obj,
        "talking",
        30,
        [
            (8, "head", (0.05, 0.0, 0.03), None),
            (16, "head", (-0.03, 0.0, -0.03), None),
            (24, "chest", (0.02, 0.0, 0.02), None),
        ],
    )
    create_action(
        arm_obj,
        "happy",
        40,
        [
            (10, "root", (0.0, 0.0, 0.0), (0.0, 0.0, 0.10)),
            (20, "root", (0.0, 0.0, 0.0), (0.0, 0.0, 0.0)),
            (15, "head", (-0.16, 0.0, 0.0), None),
            (25, "ear.L", (0.0, 0.0, 0.2), None),
            (25, "ear.R", (0.0, 0.0, -0.2), None),
        ],
    )
    create_action(
        arm_obj,
        "surprised",
        20,
        [
            (8, "head", (-0.22, 0.0, 0.0), None),
            (10, "ear.L", (0.0, 0.0, 0.30), None),
            (10, "ear.R", (0.0, 0.0, -0.30), None),
            (10, "shoulder.L", (0.0, 0.0, 0.18), None),
            (10, "shoulder.R", (0.0, 0.0, -0.18), None),
        ],
    )
    create_action(
        arm_obj,
        "sad",
        40,
        [
            (15, "head", (0.24, 0.0, 0.0), None),
            (15, "ear.L", (0.0, 0.0, -0.22), None),
            (15, "ear.R", (0.0, 0.0, 0.22), None),
            (30, "spine", (0.10, 0.0, 0.0), None),
        ],
    )
    create_action(
        arm_obj,
        "angry",
        30,
        [
            (10, "head", (0.14, 0.0, 0.0), None),
            (10, "ear.L", (0.0, 0.0, -0.30), None),
            (10, "ear.R", (0.0, 0.0, 0.30), None),
            (20, "chest", (0.12, 0.0, 0.0), None),
        ],
    )
    create_action(
        arm_obj,
        "sleeping",
        90,
        [
            (30, "head", (0.28, 0.0, 0.0), None),
            (30, "chest", (0.10, 0.0, 0.0), None),
            (60, "chest", (0.14, 0.0, 0.0), None),
        ],
    )
    create_action(
        arm_obj,
        "blink",
        12,
        [
            (4, "head", (0.02, 0.0, 0.0), None),
        ],
    )
    create_action(
        arm_obj,
        "touch_head",
        20,
        [
            (8, "head", (0.0, 0.0, 0.24), None),
            (12, "ear.L", (0.0, 0.0, 0.20), None),
            (12, "ear.R", (0.0, 0.0, -0.20), None),
        ],
    )
    create_action(
        arm_obj,
        "touch_body",
        25,
        [
            (8, "root", (0.0, 0.0, 0.0), (0.0, 0.0, 0.06)),
            (16, "head", (-0.10, 0.0, 0.0), None),
            (16, "tail.02", (0.0, 0.0, 0.25), None),
        ],
    )


def setup_render():
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 512
    scene.render.resolution_y = 512
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = False
    scene.world.color = (0.055, 0.06, 0.08)

    bpy.ops.object.camera_add(location=(0.0, -3.2, 1.35))
    camera = bpy.context.object
    camera.name = "ThumbnailCamera"
    camera.rotation_euler = (math.radians(83), 0.0, 0.0)
    camera.data.lens = 55
    scene.camera = camera

    bpy.ops.object.light_add(type="AREA", location=(2.2, -2.6, 3.2))
    key = bpy.context.object
    key.name = "KeyLight"
    key.data.energy = 700
    key.data.size = 3.0

    bpy.ops.object.light_add(type="AREA", location=(-2.5, -1.8, 2.2))
    fill = bpy.context.object
    fill.name = "FillLight"
    fill.data.energy = 300
    fill.data.size = 3.0

    bpy.ops.object.light_add(type="AREA", location=(0.0, 2.5, 3.0))
    rim = bpy.context.object
    rim.name = "RimLight"
    rim.data.energy = 500
    rim.data.size = 2.0

    # Front fill lifts the eye sockets, which otherwise read as black slits.
    bpy.ops.object.light_add(type="AREA", location=(0.0, -4.0, 1.8))
    front = bpy.context.object
    front.name = "FaceFill"
    front.data.energy = 350
    front.data.size = 2.5

    scene.render.filepath = THUMB_PATH
    scene.render.image_settings.file_format = "PNG"
    bpy.ops.render.render(write_still=True)
    print(f"Thumbnail: {THUMB_PATH}")


def export_glb():
    kwargs = dict(
        filepath=GLB_PATH,
        export_format="GLB",
        export_yup=True,
        export_apply=False,
        export_animations=True,
        export_animation_mode="ACTIONS",
        export_skins=True,
        export_morph=True,
        export_morph_animation=False,
        export_morph_normal=True,
        export_morph_tangent=False,
        export_optimize_animation_size=True,
        export_force_sampling=True,
        export_frame_range=False,
        export_anim_single_armature=True,
        export_cameras=False,
        export_lights=False,
    )
    bpy.ops.export_scene.gltf(**kwargs)
    print(f"GLB: {GLB_PATH}")


def write_metadata():
    morphs = [
        "viseme_AA", "viseme_E", "viseme_I", "viseme_O", "viseme_U",
        "viseme_M", "viseme_F", "viseme_L", "viseme_R", "viseme_S",
        "viseme_SH", "viseme_TH", "viseme_W", "viseme_REST", "viseme_BLINK",
    ]
    metadata = {
        "id": "tomiko",
        "name": "Tomiko",
        "model": "characters/tomiko/character.glb",
        "thumbnail": "characters/tomiko/thumbnail.png",
        "scale": 1.0,
        "heightMeters": 1.9,
        "animations": {
            "idle": "idle",
            "listening": "listening",
            "talking": "talking",
            "happy": "happy",
            "surprised": "surprised",
            "sad": "sad",
            "angry": "angry",
            "sleeping": "sleeping",
            "blink": "blink",
            "touch_head": "touch_head",
            "touch_body": "touch_body",
        },
        "morphTargets": {name: i for i, name in enumerate(morphs)},
        "source": "Original procedural Blender asset generated by tools/blender/create_tomiko.py",
        "license": "Original work for Tomiko",
    }
    with open(META_PATH, "w", encoding="utf-8") as f:
        json.dump(metadata, f, indent=2)
    print(f"Metadata: {META_PATH}")


def main():
    clear_scene()
    character = build_character()
    add_shape_keys(character)
    arm_obj = build_armature(character)
    build_actions(arm_obj)

    # Ensure the rest pose and neutral face are what gets exported/rendered.
    if character.data.shape_keys:
        for key_block in character.data.shape_keys.key_blocks:
            print(f"Shape key {key_block.name} value={key_block.value:.3f}")
            key_block.value = 0.0

    # Export mesh + armature only; hide cameras/lights from export via flags.
    bpy.ops.object.select_all(action="DESELECT")
    character.select_set(True)
    arm_obj.select_set(True)
    bpy.context.view_layer.objects.active = arm_obj

    export_glb()
    setup_render()
    write_metadata()
    print("DONE")


if __name__ == "__main__":
    main()
