import Foundation
import simd

/// Geometric analysis utilities for extracting measurements from 3D point clouds.
enum GeometryService {

    /// Extract ear measurements from a LiDAR point cloud of the ear.
    /// Uses PCA-based orientation estimation and bounding analysis.
    static func extractEarMeasurements(from points: [SIMD3<Float>]) -> EarParameters {
        guard points.count > 50 else { return .defaultValue }

        // 1. Center the point cloud
        let centroid = points.reduce(SIMD3<Float>.zero, +) / Float(points.count)
        let centered = points.map { $0 - centroid }

        // 2. PCA to find principal axes (ear orientation)
        let axes = principalAxes(centered)

        // 3. Project points onto principal axes
        let projections = centered.map { point -> SIMD3<Float> in
            SIMD3<Float>(
                dot(point, axes.0),
                dot(point, axes.1),
                dot(point, axes.2)
            )
        }

        // 4. Compute bounding box along principal axes
        var minProj = SIMD3<Float>(Float.infinity, Float.infinity, Float.infinity)
        var maxProj = SIMD3<Float>(-Float.infinity, -Float.infinity, -Float.infinity)
        for p in projections {
            minProj = min(minProj, p)
            maxProj = max(maxProj, p)
        }
        let extent = maxProj - minProj

        // 5. Map extents to ear parameters (in cm, points are in meters)
        // Axis 0 = longest axis (top to bottom of pinna)
        // Axis 1 = second axis (front to back of pinna)
        // Axis 2 = depth axis (ear protrusion from head)
        let pinnaHeight = extent.x * 100.0  // d5
        let pinnaWidth = extent.y * 100.0   // d7-related
        let earDepth = extent.z * 100.0     // d8-related

        // 6. Estimate concha dimensions from curvature analysis
        let conchaParams = estimateConchaDimensions(projections: projections, extent: extent)

        return EarParameters(
            d1: conchaParams.d1 * 100.0,
            d2: conchaParams.d2 * 100.0,
            d3: conchaParams.d3 * 100.0,
            d5: pinnaHeight,
            d7: pinnaWidth,
            d8: earDepth
        )
    }

    /// Compute principal axes of a centered point cloud via eigendecomposition
    /// of the covariance matrix.
    private static func principalAxes(_ points: [SIMD3<Float>]) -> (SIMD3<Float>, SIMD3<Float>, SIMD3<Float>) {
        let n = Float(points.count)

        // Build 3x3 covariance matrix
        var cov = simd_float3x3(0)
        for p in points {
            cov[0][0] += p.x * p.x
            cov[0][1] += p.x * p.y
            cov[0][2] += p.x * p.z
            cov[1][0] += p.y * p.x
            cov[1][1] += p.y * p.y
            cov[1][2] += p.y * p.z
            cov[2][0] += p.z * p.x
            cov[2][1] += p.z * p.y
            cov[2][2] += p.z * p.z
        }
        cov = cov * (1.0 / n)

        // Power iteration to find eigenvectors (sufficient for 3x3)
        let v1 = powerIteration(cov, iterations: 50)
        let deflated1 = cov - outerProduct(v1, v1) * dot(v1, cov * v1)
        let v2 = powerIteration(deflated1, iterations: 50)
        let v3 = cross(v1, v2)

        return (normalize(v1), normalize(v2), normalize(v3))
    }

    private static func powerIteration(_ matrix: simd_float3x3, iterations: Int) -> SIMD3<Float> {
        var v = SIMD3<Float>(1, 1, 1)
        for _ in 0..<iterations {
            v = matrix * v
            let len = length(v)
            if len > 1e-10 {
                v = v / len
            }
        }
        return v
    }

    private static func outerProduct(_ a: SIMD3<Float>, _ b: SIMD3<Float>) -> simd_float3x3 {
        simd_float3x3(
            SIMD3<Float>(a.x * b.x, a.y * b.x, a.z * b.x),
            SIMD3<Float>(a.x * b.y, a.y * b.y, a.z * b.y),
            SIMD3<Float>(a.x * b.z, a.y * b.z, a.z * b.z)
        )
    }

    /// Estimate concha (ear bowl) dimensions from the inner region of the point cloud.
    /// Uses depth variation to identify the concave bowl region.
    private static func estimateConchaDimensions(
        projections: [SIMD3<Float>],
        extent: SIMD3<Float>
    ) -> (d1: Float, d2: Float, d3: Float) {
        // The concha is in the lower-center region of the ear, identifiable
        // as the deepest concavity in the point cloud.

        // Filter to center 40% of the ear (where concha is)
        let centerY = extent.y / 2.0
        let centerX = extent.x * 0.4  // concha is in the lower 40% of the ear
        let radiusY = extent.y * 0.2
        let radiusX = extent.x * 0.2

        let conchaPoints = projections.filter { p in
            let inY = abs(p.y - centerY) < radiusY
            let inX = abs(p.x - centerX) < radiusX
            return inY && inX
        }

        guard conchaPoints.count > 10 else {
            // Fallback: estimate from overall ear proportions
            return (
                d1: extent.x * 0.25,  // cavum concha height ≈ 25% of pinna height
                d2: extent.x * 0.12,  // cymba concha height ≈ 12% of pinna height
                d3: extent.y * 0.35   // cavum concha width ≈ 35% of pinna width
            )
        }

        // Compute bounding box of concha region
        var minC = SIMD3<Float>(Float.infinity, Float.infinity, Float.infinity)
        var maxC = SIMD3<Float>(-Float.infinity, -Float.infinity, -Float.infinity)
        for p in conchaPoints {
            minC = min(minC, p)
            maxC = max(maxC, p)
        }
        let conchaExtent = maxC - minC

        // d1 = cavum concha height (lower bowl)
        // d2 = cymba concha height (upper bowl, roughly 40% of total concha height)
        // d3 = cavum concha width
        let totalConchaHeight = conchaExtent.x
        return (
            d1: totalConchaHeight * 0.6,
            d2: totalConchaHeight * 0.4,
            d3: conchaExtent.y
        )
    }

    // MARK: - Ellipsoid Fitting (for head measurements from LiDAR)

    /// Fit an ellipsoid to head point cloud and return (width, depth) in cm.
    static func fitHeadEllipsoid(points: [SIMD3<Float>]) -> (width: Float, depth: Float)? {
        guard points.count > 100 else { return nil }

        let centroid = points.reduce(SIMD3<Float>.zero, +) / Float(points.count)
        let centered = points.map { $0 - centroid }
        let axes = principalAxes(centered)

        // Project onto principal axes and get extents
        var extents = SIMD3<Float>.zero
        for point in centered {
            let proj = SIMD3<Float>(
                abs(dot(point, axes.0)),
                abs(dot(point, axes.1)),
                abs(dot(point, axes.2))
            )
            extents = max(extents, proj)
        }

        // Sort extents: largest two are width and depth
        let sorted = [extents.x, extents.y, extents.z].sorted(by: >)
        // Full diameter = 2 * semi-axis, convert m → cm
        let width = sorted[0] * 2.0 * 100.0
        let depth = sorted[1] * 2.0 * 100.0

        return (width: width, depth: depth)
    }
}
