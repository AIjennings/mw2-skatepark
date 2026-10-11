use crate::drawsurf::scene_depth::{SCENE_DEPTH_FORMAT, SceneDepthTexture};
use bevy::core_pipeline::core_3d::main_opaque_pass_3d;
use bevy::core_pipeline::{Core3d, Core3dSystems};
use bevy::mesh::VertexBufferLayout;
use bevy::prelude::*;
use bevy::render::RenderStartup;
use bevy::render::render_resource::binding_types::uniform_buffer;
use bevy::render::render_resource::{
    BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, Buffer,
    BufferInitDescriptor, BufferUsages, CachedRenderPipelineId, ColorTargetState, ColorWrites,
    CompareFunction, DepthStencilState, FragmentState, FrontFace, IndexFormat,
    MultisampleState, PipelineCache, PrimitiveState, RenderPassDescriptor,
    RenderPipelineDescriptor, ShaderStages, SpecializedRenderPipeline, SpecializedRenderPipelines,
    StoreOp, TextureFormat, VertexAttribute, VertexFormat, VertexState, VertexStepMode,
};
use bevy::render::renderer::{RenderContext, RenderDevice, ViewQuery};
use bevy::render::view::{
    ExtractedView, Msaa, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms,
};
use bevy::render::{Render, RenderSystems};
use bevy::shader::Shader;

const SHADER_PATH: &str = "embedded://render_gpu/drawsurf/skate_park.wgsl";

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkateParkGpuVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

#[derive(Resource, Clone, Debug, Default)]
pub struct ExtractedSkateParkMesh {
    pub vertices: Vec<SkateParkGpuVertex>,
    pub indices: Vec<u32>,
    pub revision: u64,
}

#[derive(Resource, Default)]
struct SkateParkGpu {
    vertex_buffer: Option<Buffer>,
    index_buffer: Option<Buffer>,
    index_count: u32,
    revision: u64,
}

#[derive(Resource)]
struct SkateParkPipeline {
    view_layout: BindGroupLayoutDescriptor,
    shader: Handle<Shader>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SkateParkPipelineKey {
    target: TextureFormat,
    samples: u32,
}

impl SpecializedRenderPipeline for SkateParkPipeline {
    type Key = SkateParkPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("iw4_skate_park".into()),
            layout: vec![self.view_layout.clone()],
            immediate_size: 0,
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: Vec::new(),
                entry_point: Some("vertex".into()),
                buffers: vec![VertexBufferLayout {
                    array_stride: size_of::<SkateParkGpuVertex>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 12,
                            shader_location: 1,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x4,
                            offset: 24,
                            shader_location: 2,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 40,
                            shader_location: 3,
                        },
                    ],
                }],
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: Vec::new(),
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format: key.target,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                front_face: FrontFace::Cw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: SCENE_DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::GreaterEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: MultisampleState {
                count: key.samples,
                ..Default::default()
            },
            zero_initialize_workgroup_memory: false,
        }
    }
}

fn init_skate_park_pipeline(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(SkateParkPipeline {
        view_layout: BindGroupLayoutDescriptor::new(
            "iw4_skate_park_view",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (uniform_buffer::<ViewUniform>(true),),
            ),
        ),
        shader: asset_server.load(SHADER_PATH),
    });
}

fn upload_skate_park(
    mut gpu: ResMut<SkateParkGpu>,
    source: Option<Res<ExtractedSkateParkMesh>>,
    device: Res<RenderDevice>,
) {
    let Some(source) = source else {
        return;
    };
    if source.revision == gpu.revision && gpu.vertex_buffer.is_some() {
        return;
    }
    gpu.revision = source.revision;
    gpu.index_count = source.indices.len() as u32;
    if source.vertices.is_empty() || source.indices.is_empty() {
        gpu.vertex_buffer = None;
        gpu.index_buffer = None;
        return;
    }
    gpu.vertex_buffer = Some(device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("iw4_skate_park_vb"),
        contents: bytemuck::cast_slice(source.vertices.as_slice()),
        usage: BufferUsages::VERTEX,
    }));
    gpu.index_buffer = Some(device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("iw4_skate_park_ib"),
        contents: bytemuck::cast_slice(source.indices.as_slice()),
        usage: BufferUsages::INDEX,
    }));
}

#[derive(Component)]
struct SkateParkViewBindGroup {
    bind_group: BindGroup,
    pipeline_id: CachedRenderPipelineId,
}

fn prepare_skate_park_views(
    mut commands: Commands,
    pipeline: Option<Res<SkateParkPipeline>>,
    mut specialized: ResMut<SpecializedRenderPipelines<SkateParkPipeline>>,
    cache: Res<PipelineCache>,
    device: Res<RenderDevice>,
    uniforms: Res<ViewUniforms>,
    views: Query<(Entity, &ExtractedView, &Msaa)>,
) {
    let (Some(pipeline), Some(binding)) = (pipeline, uniforms.uniforms.binding()) else {
        return;
    };
    for (entity, view, msaa) in &views {
        let pipeline_id = specialized.specialize(
            &cache,
            &pipeline,
            SkateParkPipelineKey {
                target: view.target_format,
                samples: msaa.samples(),
            },
        );
        let bind_group = device.create_bind_group(
            "iw4_skate_park_view_bg",
            &cache.get_bind_group_layout(&pipeline.view_layout),
            &BindGroupEntries::single(binding.clone()),
        );
        commands.entity(entity).insert(SkateParkViewBindGroup {
            bind_group,
            pipeline_id,
        });
    }
}

fn draw_skate_park(
    view: ViewQuery<(
        &ViewTarget,
        &SceneDepthTexture,
        &ViewUniformOffset,
        &SkateParkViewBindGroup,
    )>,
    gpu: Res<SkateParkGpu>,
    cache: Res<PipelineCache>,
    mut context: RenderContext,
) {
    let (Some(vb), Some(ib)) = (gpu.vertex_buffer.as_ref(), gpu.index_buffer.as_ref()) else {
        return;
    };
    if gpu.index_count == 0 {
        return;
    }
    let (target, depth, view_offset, view_bind) = view.into_inner();
    let Some(render_pipeline) = cache.get_render_pipeline(view_bind.pipeline_id) else {
        return;
    };
    let attachments = [Some(target.get_color_attachment())];
    let mut pass = context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("iw4_skate_park_pass"),
        color_attachments: &attachments,
        depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });

    pass.set_render_pipeline(render_pipeline);
    pass.set_bind_group(0, &view_bind.bind_group, &[view_offset.offset]);
    pass.set_vertex_buffer(0, vb.slice(..));
    pass.set_index_buffer(ib.slice(..), IndexFormat::Uint32);
    pass.draw_indexed(0..gpu.index_count, 0, 0..1);
}

pub(super) fn register(app: &mut App) {
    bevy::asset::embedded_asset!(app, "skate_park.wgsl");
    let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) else {
        return;
    };
    render_app
        .init_resource::<ExtractedSkateParkMesh>()
        .init_resource::<SkateParkGpu>()
        .init_resource::<SpecializedRenderPipelines<SkateParkPipeline>>()
        .add_systems(RenderStartup, init_skate_park_pipeline)
        .add_systems(
            Render,
            (
                upload_skate_park.in_set(RenderSystems::PrepareResources),
                prepare_skate_park_views.in_set(RenderSystems::PrepareBindGroups),
            ),
        )
        .add_systems(
            Core3d,
            draw_skate_park
                .in_set(Core3dSystems::MainPass)
                .after(main_opaque_pass_3d)
                .after(super::draw::ExactColourDrawSet),
        );
}
