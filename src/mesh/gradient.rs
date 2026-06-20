use glam::DVec2;

use super::types::TriangleMesh;

pub fn compute_gradient_field(
    mesh: &TriangleMesh,
    u_field: &[f64],
) -> Result<Vec<DVec2>, String> {
    if u_field.len() != mesh.vertices.len() {
        return Err(format!(
            "u_field length {} does not match vertex count {}",
            u_field.len(),
            mesh.vertices.len()
        ));
    }
    let mut gradients: Vec<DVec2> = Vec::with_capacity(mesh.triangles.len());
    for tri in &mesh.triangles {
        let i = tri[0];
        let j = tri[1];
        let k = tri[2];
        let vi = mesh.vertices[i];
        let vj = mesh.vertices[j];
        let vk = mesh.vertices[k];

        let bi = vj.y - vk.y;
        let ci = vk.x - vj.x;
        let bj = vk.y - vi.y;
        let cj = vi.x - vk.x;
        let bk = vi.y - vj.y;
        let ck = vj.x - vi.x;

        let area2 = bi * cj - bj * ci;
        if area2.abs() < 1e-30 {
            gradients.push(DVec2::ZERO);
            continue;
        }
        let ui = u_field[i];
        let uj = u_field[j];
        let uk = u_field[k];

        let gx = (bi * ui + bj * uj + bk * uk) / area2;
        let gy = (ci * ui + cj * uj + ck * uk) / area2;
        gradients.push(DVec2::new(gx, gy));
    }
    Ok(gradients)
}
