# SPDX-License-Identifier: MIT OR Apache-2.0
# Usage: python3 tools/brand-icons.py assets/brand/manuscript 32 256 512 1024
# Rasterise the mark (same shapes as mark.svg) with 4x4 supersampling; stdlib only.
import struct, zlib, sys
def rr(px,py,x,y,w,h,r):
    if px<x or py<y or px>x+w or py>y+h: return False
    cx=min(max(px,x+r),x+w-r); cy=min(max(py,y+r),y+h-r)
    return (px-cx)**2+(py-cy)**2<=r*r
def page(px,py):
    return 17<=px<=47 and 12<=py<=52 and not (px>39 and py<20 and (px-39)>(py-12))
def corner(px,py): return 39<=px<=47 and 12<=py<=20 and (px-39)>=(py-12)
def color(px,py):
    if (px-40)**2+(py-41.5)**2<=4: return (0xC9,0x97,0x3B,255)
    for (x,y,w,h) in [(22,26,20,3),(22,33,20,3),(22,40,13,3)]:
        if rr(px,py,x,y,w,h,1.5): return (0x7A,0x2E,0x3A,255)
    if corner(px,py): return (0xE2,0xCF,0xA8,255)
    if page(px,py): return (0xFB,0xF9,0xF5,255)
    if rr(px,py,2,2,60,60,13): return (0x7A,0x2E,0x3A,255)
    return (0,0,0,0)
def png(n,path):
    s=4; rows=[]
    for j in range(n):
        row=bytearray([0])
        for i in range(n):
            acc=[0,0,0,0]
            for a in range(s):
                for b in range(s):
                    c=color((i+(a+.5)/s)*64/n,(j+(b+.5)/s)*64/n)
                    for k in range(3): acc[k]+=c[k]*c[3]
                    acc[3]+=c[3]
            al=acc[3]
            row+=bytes([round(acc[0]/al) if al else 0,round(acc[1]/al) if al else 0,round(acc[2]/al) if al else 0,round(al/(s*s))])
        rows.append(bytes(row))
    def chunk(t,d): return struct.pack('>I',len(d))+t+d+struct.pack('>I',zlib.crc32(t+d)&0xffffffff)
    data=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',n,n,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(b''.join(rows),9))+chunk(b'IEND',b'')
    open(path,'wb').write(data)
for n in map(int,sys.argv[2:]): png(n,f"{sys.argv[1]}/icon-{n}.png")
