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

---@alias CurrentRoom number

---@class Damage
---@field amount number

---@alias FacingDirection table

---@class Global
--- Marker component

---@class Glow
---@field color vec3
---@field intensity number
---@field brightness number
---@field emission number
---@field sprite_id number

---@alias Grounded boolean

---@class Interactable
---@field range number

---@class Kinematic
--- Marker component

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

---@alias Parent table

---@class PhysicsBody
--- Marker component

---@class Player
--- Marker component

---@class PlayerProxy
--- Marker component

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
---@field Animation string
---@field AudioSource string
---@field Children string
---@field Collider string
---@field CurrentFrame string
---@field CurrentRoom string
---@field Damage string
---@field FacingDirection string
---@field Global string
---@field Glow string
---@field Grounded string
---@field Interactable string
---@field Kinematic string
---@field Layer string
---@field Light string
---@field Name string
---@field Parent string
---@field PhysicsBody string
---@field Player string
---@field PlayerProxy string
---@field RoomCamera string
---@field Script string
---@field Solid string
---@field SpeechBubble string
---@field Sprite string
---@field SubPixel string
---@field Transform string
---@field Velocity string
---@field Walkable string

local C = {}

C.Animation = "Animation"
C.AudioSource = "AudioSource"
C.Children = "Children"
C.Collider = "Collider"
C.CurrentFrame = "CurrentFrame"
C.CurrentRoom = "CurrentRoom"
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
