-- Auto-generated. Do not edit.
-- bishop-owner: shared-engine
---@meta
---@alias vec2 { x: number, y: number }
---@alias vec3 { x: number, y: number, z: number }

---@class Animation
---@field clips table
---@field variant table
---@field current table
---@field states table
---@field sprite_cache table
---@field flip_x boolean
---@field speed_multiplier number

---@class AudioSource
---@field groups table
---@field current table
---@field runtime_volume number

---@class Children
---@field entities table

---@class Collider
---@field width number
---@field height number

---@class CurrentFrame
---@field clip_id number
---@field col number
---@field row number
---@field offset vec2
---@field sprite_id number
---@field frame_size vec2
---@field flip_x boolean

---@class Damage
---@field amount number

---@alias FacingDirection Direction

--- Marker component
---@class Global

---@class Glow
---@field color vec3
---@field intensity number
---@field brightness number
---@field emission number
---@field sprite_id number

---@alias Grounded boolean

---@class Interactable
---@field range number

--- Marker component
---@class Kinematic

---@class Layer
---@field z number

---@class Light
---@field pos vec2
---@field color vec3
---@field intensity number
---@field radius number
---@field spread number
---@field alpha number
---@field brightness number

---@alias Name string

---@alias Parent Entity

--- Marker component
---@class PhysicsBody

--- Marker component
---@class Player

--- Marker component
---@class PlayerProxy

---@class RoomCamera
---@field zoom vec2
---@field room_id number
---@field zoom_mode table
---@field camera_mode table

---@class Script
---@field script_id number
---@field data table

---@alias Solid boolean

---@class SpeechBubble
---@field text string
---@field timer number
---@field color table
---@field offset table
---@field font_size table
---@field max_width table
---@field show_background boolean
---@field background_color table

---@class Sprite
---@field sprite number

---@class SubPixel
---@field x number
---@field y number

---@class Transform
---@field visible boolean
---@field position vec2
---@field pivot table

---@class Velocity
---@field x number
---@field y number

---@alias Walkable boolean

---@class ComponentId
---@field Animation "Animation"
---@field AudioSource "AudioSource"
---@field Children "Children"
---@field Collider "Collider"
---@field CurrentFrame "CurrentFrame"
---@field Damage "Damage"
---@field FacingDirection "FacingDirection"
---@field Global "Global"
---@field Glow "Glow"
---@field Grounded "Grounded"
---@field Interactable "Interactable"
---@field Kinematic "Kinematic"
---@field Layer "Layer"
---@field Light "Light"
---@field Name "Name"
---@field Parent "Parent"
---@field PhysicsBody "PhysicsBody"
---@field Player "Player"
---@field PlayerProxy "PlayerProxy"
---@field RoomCamera "RoomCamera"
---@field Script "Script"
---@field Solid "Solid"
---@field SpeechBubble "SpeechBubble"
---@field Sprite "Sprite"
---@field SubPixel "SubPixel"
---@field Transform "Transform"
---@field Velocity "Velocity"
---@field Walkable "Walkable"

local C = {}

C.Animation = "Animation"
C.AudioSource = "AudioSource"
C.Children = "Children"
C.Collider = "Collider"
C.CurrentFrame = "CurrentFrame"
C.Damage = "Damage"
C.FacingDirection = "FacingDirection"
C.Global = "Global"
C.Glow = "Glow"
C.Grounded = "Grounded"
C.Interactable = "Interactable"
C.Kinematic = "Kinematic"
C.Layer = "Layer"
C.Light = "Light"
C.Name = "Name"
C.Parent = "Parent"
C.PhysicsBody = "PhysicsBody"
C.Player = "Player"
C.PlayerProxy = "PlayerProxy"
C.RoomCamera = "RoomCamera"
C.Script = "Script"
C.Solid = "Solid"
C.SpeechBubble = "SpeechBubble"
C.Sprite = "Sprite"
C.SubPixel = "SubPixel"
C.Transform = "Transform"
C.Velocity = "Velocity"
C.Walkable = "Walkable"

return C
