/* Storm Gate synthetic test: render a triangle with Direct3D 11 into an
 * offscreen texture, read it back and check known pixels (a tiny golden
 * image test). Under Storm Gate this runs through DXMT -> Metal.
 *
 * Exit codes: 0 pass, 2 device, 3 shader, 4 resources, 5 readback, 6 pixels.
 * SPDX-License-Identifier: Apache-2.0 OR MIT */
#define COBJMACROS
#define INITGUID
#include <stdio.h>
#include <string.h>
#include <windows.h>
#include <d3d11.h>
#include <d3dcompiler.h>

#define W 64
#define H 64

static const char shader_src[] =
    "struct VSOut { float4 pos : SV_Position; };\n"
    "VSOut vs_main(float2 pos : POSITION) {\n"
    "    VSOut o; o.pos = float4(pos, 0.0, 1.0); return o;\n"
    "}\n"
    "float4 ps_main(VSOut i) : SV_Target { return float4(1.0, 0.0, 0.0, 1.0); }\n";

typedef HRESULT(WINAPI *compile_fn)(const void *, SIZE_T, const char *, const D3D_SHADER_MACRO *,
                                    ID3DInclude *, const char *, const char *, UINT, UINT,
                                    ID3DBlob **, ID3DBlob **);

static ID3DBlob *compile(compile_fn fn, const char *entry, const char *target)
{
    ID3DBlob *code = NULL, *errors = NULL;
    HRESULT hr = fn(shader_src, sizeof(shader_src) - 1, "triangle.hlsl", NULL, NULL, entry,
                    target, 0, 0, &code, &errors);
    if (FAILED(hr))
    {
        printf("D3DCompile(%s) failed: %#lx %s\n", entry, hr,
               errors ? (const char *)ID3D10Blob_GetBufferPointer(errors) : "");
        return NULL;
    }
    if (errors)
        ID3D10Blob_Release(errors);
    return code;
}

static int pixel_is(const BYTE *px, BYTE r, BYTE g, BYTE b)
{
    return abs(px[0] - r) <= 2 && abs(px[1] - g) <= 2 && abs(px[2] - b) <= 2;
}

int main(void)
{
    static const float verts[] = { 0.0f, 0.8f, 0.8f, -0.8f, -0.8f, -0.8f };
    static const float clear[4] = { 0.0f, 0.0f, 1.0f, 1.0f };
    D3D_FEATURE_LEVEL level;
    ID3D11Device *dev;
    ID3D11DeviceContext *ctx;
    ID3D11Texture2D *rt, *staging;
    ID3D11RenderTargetView *rtv;
    ID3D11VertexShader *vs;
    ID3D11PixelShader *ps;
    ID3D11InputLayout *layout;
    ID3D11Buffer *vb;
    ID3DBlob *vs_code, *ps_code;
    D3D11_TEXTURE2D_DESC td = { 0 };
    D3D11_BUFFER_DESC bd = { 0 };
    D3D11_SUBRESOURCE_DATA init = { verts };
    D3D11_INPUT_ELEMENT_DESC elem = { "POSITION", 0, DXGI_FORMAT_R32G32_FLOAT, 0, 0,
                                      D3D11_INPUT_PER_VERTEX_DATA, 0 };
    D3D11_VIEWPORT vp = { 0, 0, W, H, 0, 1 };
    D3D11_MAPPED_SUBRESOURCE map;
    UINT stride = 8, offset = 0;
    const BYTE *center, *corner;
    compile_fn d3dcompile;
    HMODULE compiler;
    HRESULT hr;

    hr = D3D11CreateDevice(NULL, D3D_DRIVER_TYPE_HARDWARE, NULL, 0, NULL, 0, D3D11_SDK_VERSION,
                           &dev, &level, &ctx);
    if (FAILED(hr))
    {
        printf("D3D11CreateDevice failed: %#lx\n", hr);
        return 2;
    }
    printf("D3D11 initialization: PASS (feature level %#x)\n", level);

    if (!(compiler = LoadLibraryA("d3dcompiler_47.dll"))
        || !(d3dcompile = (compile_fn)(void *)GetProcAddress(compiler, "D3DCompile")))
    {
        printf("d3dcompiler_47.dll unavailable\n");
        return 3;
    }
    if (!(vs_code = compile(d3dcompile, "vs_main", "vs_4_0"))
        || !(ps_code = compile(d3dcompile, "ps_main", "ps_4_0")))
        return 3;

    td.Width = W;
    td.Height = H;
    td.MipLevels = 1;
    td.ArraySize = 1;
    td.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    td.SampleDesc.Count = 1;
    td.Usage = D3D11_USAGE_DEFAULT;
    td.BindFlags = D3D11_BIND_RENDER_TARGET;
    if (FAILED(ID3D11Device_CreateTexture2D(dev, &td, NULL, &rt)))
        return 4;
    td.Usage = D3D11_USAGE_STAGING;
    td.BindFlags = 0;
    td.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
    if (FAILED(ID3D11Device_CreateTexture2D(dev, &td, NULL, &staging)))
        return 4;
    if (FAILED(ID3D11Device_CreateRenderTargetView(dev, (ID3D11Resource *)rt, NULL, &rtv)))
        return 4;
    bd.ByteWidth = sizeof(verts);
    bd.Usage = D3D11_USAGE_IMMUTABLE;
    bd.BindFlags = D3D11_BIND_VERTEX_BUFFER;
    if (FAILED(ID3D11Device_CreateBuffer(dev, &bd, &init, &vb)))
        return 4;
    if (FAILED(ID3D11Device_CreateVertexShader(dev, ID3D10Blob_GetBufferPointer(vs_code),
                                               ID3D10Blob_GetBufferSize(vs_code), NULL, &vs))
        || FAILED(ID3D11Device_CreatePixelShader(dev, ID3D10Blob_GetBufferPointer(ps_code),
                                                 ID3D10Blob_GetBufferSize(ps_code), NULL, &ps))
        || FAILED(ID3D11Device_CreateInputLayout(dev, &elem, 1, ID3D10Blob_GetBufferPointer(vs_code),
                                                 ID3D10Blob_GetBufferSize(vs_code), &layout)))
        return 3;

    ID3D11DeviceContext_OMSetRenderTargets(ctx, 1, &rtv, NULL);
    ID3D11DeviceContext_RSSetViewports(ctx, 1, &vp);
    ID3D11DeviceContext_ClearRenderTargetView(ctx, rtv, clear);
    ID3D11DeviceContext_IASetInputLayout(ctx, layout);
    ID3D11DeviceContext_IASetPrimitiveTopology(ctx, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
    ID3D11DeviceContext_IASetVertexBuffers(ctx, 0, 1, &vb, &stride, &offset);
    ID3D11DeviceContext_VSSetShader(ctx, vs, NULL, 0);
    ID3D11DeviceContext_PSSetShader(ctx, ps, NULL, 0);
    ID3D11DeviceContext_Draw(ctx, 3, 0);
    ID3D11DeviceContext_CopyResource(ctx, (ID3D11Resource *)staging, (ID3D11Resource *)rt);

    if (FAILED(ID3D11DeviceContext_Map(ctx, (ID3D11Resource *)staging, 0, D3D11_MAP_READ, 0, &map)))
    {
        printf("Map failed\n");
        return 5;
    }
    center = (const BYTE *)map.pData + (H / 2) * map.RowPitch + (W / 2) * 4;
    corner = (const BYTE *)map.pData + 1 * map.RowPitch + 1 * 4;
    printf("center %02x%02x%02x corner %02x%02x%02x\n", center[0], center[1], center[2], corner[0],
           corner[1], corner[2]);
    if (!pixel_is(center, 255, 0, 0) || !pixel_is(corner, 0, 0, 255))
    {
        printf("frame check: FAIL\n");
        return 6;
    }
    ID3D11DeviceContext_Unmap(ctx, (ID3D11Resource *)staging, 0);
    printf("frame check: PASS\n");
    printf("STORMGATE-TEST-PASS d3d11-triangle\n");
    fflush(stdout);
    return 0;
}
