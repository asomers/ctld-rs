#! /bin/sh

CRATEDIR=`dirname $0`/..
SRC_BASE=/usr/home/somers/src/freebsd.org/src

cat > src/ffi.rs << HERE
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused)]
HERE

bindgen --allowlist-type 'ctl_lun_list' \
	--allowlist-item 'CTL_DEFAULT_DEV' \
	--rustified-enum 'ctl_lun_list_status' \
	${CRATEDIR}/bindgen/wrapper.h -- \
	-I${SRC_BASE} >> ${CRATEDIR}/src/ffi.rs
rustfmt ${CRATEDIR}/src/ffi.rs


