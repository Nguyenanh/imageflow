use super::internal_prelude::*;

pub static  FLIP_V_PRIMITIVE: FlipVerticalMutNodeDef = FlipVerticalMutNodeDef{} ;
pub static  FLIP_H_PRIMITIVE: FlipHorizontalMutNodeDef = FlipHorizontalMutNodeDef{};

pub static  FLIP_V: MutProtect<FlipVerticalMutNodeDef> = MutProtect{ node: &FLIP_V_PRIMITIVE, fqn: "imazen.flip_vertical"};
pub static  FLIP_H: MutProtect<FlipHorizontalMutNodeDef> = MutProtect{ node: &FLIP_H_PRIMITIVE, fqn: "imazen.flip_horizontal"};

pub static  APPLY_ORIENTATION: ApplyOrientationDef = ApplyOrientationDef{};


lazy_static! {
    pub static ref NO_OP: NodeDefinition = no_op_def();

    pub static ref ROTATE_90: NodeDefinition = rotate90_def();
     pub static ref ROTATE_180: NodeDefinition = rotate180_def();
    pub static ref ROTATE_270: NodeDefinition = rotate270_def();


    pub static ref TRANSPOSE: NodeDefinition = transpose_def();

    pub static ref TRANSPOSE_MUT: NodeDefinition = transpose_mut_def();
}



#[derive(Debug,Clone)]
pub struct ApplyOrientationDef;
impl NodeDef for ApplyOrientationDef{
    fn as_one_input_expand(&self) -> Option<&NodeDefOneInputExpand>{
        Some(self)
    }
}
impl NodeDefOneInputExpand for ApplyOrientationDef{
    fn fqn(&self) -> &'static str{
        "imazen.apply_orientation"
    }
    fn estimate(&self, p: &NodeParams, input: FrameEstimate) -> NResult<FrameEstimate> {
        if let &NodeParams::Json(s::Node::ApplyOrientation { flag }) = p {
            input.map_frame(|info| {
                let swap = flag >= 5 && flag <= 8;
                Ok(FrameInfo {
                    w: if swap {
                        info.h
                    } else { info.w },
                    h: if swap {
                        info.w
                    } else { info.h },
                    ..info
                })
            })
        } else {
            Err(nerror!(ErrorKind::NodeParamsMismatch, "Need ApplyOrientation, got {:?}", p))
        }
    }

    fn expand(&self, ctx: &mut OpCtxMut, ix: NodeIndex, p: NodeParams, parent: FrameInfo) -> NResult<()>{
        if let NodeParams::Json(s::Node::ApplyOrientation { flag }) = p {
            let replacement_nodes: Vec<&'static NodeDef> = match flag {
                7 => vec![&*ROTATE_180, &*TRANSPOSE],
                8 => vec![&*ROTATE_90],
                6 => vec![&*ROTATE_270],
                5 => vec![&*TRANSPOSE],
                4 => vec![&FLIP_V],
                3 => vec![&*ROTATE_180],
                2 => vec![&FLIP_H],
                _ => vec![],
            };
            ctx.replace_node(ix,
                             replacement_nodes.iter()
                                 .map(|v| Node::n(*v, NodeParams::None))
                                 .collect());
            Ok(())
        } else {
            Err(nerror!(ErrorKind::NodeParamsMismatch, "Need ApplyOrientation, got {:?}", p))
        }
    }
}

fn transpose_def() -> NodeDefinition {
    NodeDefinition {
        fqn: "imazen.transpose",
        name: "Transpose",
        fn_estimate: Some(NodeDefHelpers::rotate_frame_info),
        fn_flatten_pre_optimize: Some({
            fn f(ctx: &mut OpCtxMut, ix: NodeIndex) {
                match ctx.first_parent_input_weight(ix).unwrap().frame_est {
                    FrameEstimate::Some(FrameInfo { w, h, fmt, alpha_meaningful }) => {
                            let canvas_params = s::Node::CreateCanvas {
                                w: h as usize,
                                h: w as usize,
                                format: s::PixelFormat::from(fmt),
                                color: s::Color::Transparent,
                            };
                            let canvas = ctx.graph
                                .add_node(Node::n(&CREATE_CANVAS,
                                                    NodeParams::Json(canvas_params)));
                            let copy = ctx.graph
                                .add_node(Node::new(&TRANSPOSE_MUT, NodeParams::None));
                            ctx.graph.add_edge(canvas, copy, EdgeKind::Canvas).unwrap();
                            ctx.replace_node_with_existing(ix, copy);

                    }
                    _ => panic!(""),
                }
            }
            f
        }),

        ..Default::default()
    }
}

fn transpose_mut_def() -> NodeDefinition {
    NodeDefinition {
        fqn: "imazen.transpose_to_canvas",
        name: "transpose_to_canvas",
        inbound_edges: EdgesIn::OneInputOneCanvas,
        description: "Transpose To",
        fn_estimate: Some(NodeDefHelpers::copy_frame_est_from_first_canvas),
        fn_execute: Some({
            fn f(ctx: &mut OpCtxMut, ix: NodeIndex) {
                let input: *mut ::ffi::BitmapBgra =
                    ctx.first_parent_result_frame(ix, EdgeKind::Input).unwrap();
                let canvas: *mut ::ffi::BitmapBgra =
                    ctx.first_parent_result_frame(ix, EdgeKind::Canvas).unwrap();

                unsafe {
                    if (*input).fmt != (*canvas).fmt {
                        panic!("Can't copy between bitmaps with different pixel formats")
                    }
                    if input == canvas {
                        panic!("Canvas and input must be different bitmaps for transpose to work!")
                    }

                    if !::ffi::flow_bitmap_bgra_transpose(ctx.flow_c(), input, canvas) {
                        panic!("Failed to transpose bitmap")
                    }

                    ctx.weight_mut(ix).result = NodeResult::Frame(canvas);
                }
            }
            f
        }),
        ..Default::default()
    }
}
fn no_op_def() -> NodeDefinition {
    NodeDefinition {
        fqn: "imazen.noop",
        name: "NoOp",
        description: "Does nothing; pass-through node",
        fn_estimate: Some(NodeDefHelpers::copy_frame_est_from_first_input),
        fn_flatten_pre_optimize: Some(NodeDefHelpers::delete_node_and_snap_together),
        ..Default::default()
    }
}

#[derive(Debug, Clone)]
pub struct FlipVerticalMutNodeDef;
impl NodeDef for FlipVerticalMutNodeDef{
    fn as_one_mutate_bitmap(&self) -> Option<&NodeDefMutateBitmap>{
        Some(self)
    }
}
impl NodeDefMutateBitmap for FlipVerticalMutNodeDef{
    fn fqn(&self) -> &'static str{
        "imazen.flip_vertical_mutate"
    }
    fn mutate(&self, c: &Context, bitmap: &mut BitmapBgra,  p: &NodeParams) -> NResult<()>{
        unsafe {
            if !::ffi::flow_bitmap_bgra_flip_vertical(c.flow_c(), bitmap as *mut BitmapBgra){
                return Err(nerror!(ErrorKind::CError(c.error().c_error())))
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct FlipHorizontalMutNodeDef;
impl NodeDef for FlipHorizontalMutNodeDef{
    fn as_one_mutate_bitmap(&self) -> Option<&NodeDefMutateBitmap>{
        Some(self)
    }
}
impl NodeDefMutateBitmap for FlipHorizontalMutNodeDef{
    fn fqn(&self) -> &'static str{
        "imazen.flip_vertical_mutate"
    }
    fn mutate(&self, c: &Context, bitmap: &mut BitmapBgra,  p: &NodeParams) -> NResult<()>{
        unsafe {
            if !::ffi::flow_bitmap_bgra_flip_horizontal(c.flow_c(), bitmap as *mut BitmapBgra){
                return Err(nerror!(ErrorKind::CError(c.error().c_error())))
            }
        }
        Ok(())
    }
}

fn rotate90_def() -> NodeDefinition {
    NodeDefinition {
        fqn: "imazen.rot90",
        name: "Rot90",
        fn_estimate: Some(NodeDefHelpers::rotate_frame_info),
        fn_flatten_pre_optimize: Some({
            fn f(ctx: &mut OpCtxMut, ix: NodeIndex) {
                ctx.replace_node(ix,
                                 vec![
                Node::new(&TRANSPOSE, NodeParams::None),
                Node::n(&FLIP_V, NodeParams::None),
                ]);
            }
            f
        }),
        ..Default::default()
    }
}
fn rotate180_def() -> NodeDefinition {
    NodeDefinition {
        fqn: "imazen.rot180",
        name: "Rot180",
        fn_estimate: Some(NodeDefHelpers::copy_frame_est_from_first_input),
        fn_flatten_pre_optimize: Some({
            fn f(ctx: &mut OpCtxMut, ix: NodeIndex) {
                ctx.replace_node(ix,
                                 vec![
                Node::n(&FLIP_V as &NodeDef, NodeParams::None),
                Node::n(&FLIP_H, NodeParams::None),
                ]);
            }
            f
        }),
        ..Default::default()
    }
}

fn rotate270_def() -> NodeDefinition {
    NodeDefinition {
        fqn: "imazen.rot270",
        name: "Rot270",
        fn_estimate: Some(NodeDefHelpers::rotate_frame_info),
        fn_flatten_pre_optimize: Some({
            fn f(ctx: &mut OpCtxMut, ix: NodeIndex) {
                ctx.replace_node(ix,
                                 vec![
                Node::n(&FLIP_V, NodeParams::None),
                Node::new(&TRANSPOSE, NodeParams::None),
                ]);
            }
            f
        }),
        ..Default::default()
    }
}

