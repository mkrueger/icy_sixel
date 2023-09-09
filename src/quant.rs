#![allow(clippy::erasing_op)]
 /*****************************************************************************
  *
  * quantization
  *
  *****************************************************************************/

use std::vec;


 /* 
 typedef struct box* boxVector;
 struct box {
     unsigned int ind;
     unsigned int colors;
     unsigned int sum;
 };
 
 typedef unsigned long sample;
 typedef sample * tuple;
 
 struct tupleint {
     /* An ordered pair of a tuple value and an integer, such as you
        would find in a tuple table or tuple hash.
        Note that this is a variable length structure.
     */
     unsigned int value;
     sample tuple[1];
     /* This is actually a variable size array -- its size is the
        depth of the tuple in question.  Some compilers do not let us
        declare a variable length array.
     */
 };
 typedef struct tupleint ** tupletable;
 
 typedef struct {
     unsigned int size;
     tupletable table;
 } tupletable2;
 
 static unsigned int compareplanePlane;

 */
     /* This is a parameter to compareplane().  We use this global variable
        so that compareplane() can be called by qsort(), to compare two
        tuples.  qsort() doesn't pass any arguments except the two tuples.
     */
/* 
 static int
 compareplane(const void * const arg1,
              const void * const arg2)
 {
     int lhs, rhs;
 
     typedef const struct tupleint * const * const sortarg;
     sortarg comparandPP  = (sortarg) arg1;
     sortarg comparatorPP = (sortarg) arg2;
     lhs = (int)(*comparandPP)->tuple[compareplanePlane];
     rhs = (int)(*comparatorPP)->tuple[compareplanePlane];
 
     return lhs - rhs;
 }
 
 
 static int
 sumcompare(const void * const b1, const void * const b2)
 {
     return (int)((boxVector)b2)->sum - (int)((boxVector)b1)->sum;
 }
 
 
 static SIXELSTATUS
 alloctupletable(
     tupletable          /* out */ *result,
     unsigned int const  /* in */  depth,
     unsigned int const  /* in */  size,
     sixel_allocator_t   /* in */  *allocator)
 {
     SIXELSTATUS status = SIXEL_FALSE;
     enum { message_buffer_size = 256 };
     char message[message_buffer_size];
     int nwrite;
     unsigned int mainTableSize;
     unsigned int tupleIntSize;
     unsigned int allocSize;
     void * pool;
     tupletable tbl;
     unsigned int i;
 
     if (UINT_MAX / sizeof(struct tupleint) < size) {
         nwrite = sprintf(message,
                          "size %u is too big for arithmetic",
                          size);
         if (nwrite > 0) {
             sixel_helper_set_additional_message(message);
         }
         status = SIXEL_RUNTIME_ERROR;
         goto end;
     }
 
     mainTableSize = size * sizeof(struct tupleint *);
     tupleIntSize = sizeof(struct tupleint) - sizeof(sample)
         + depth * sizeof(sample);
 
     /* To save the enormous amount of time it could take to allocate
        each individual tuple, we do a trick here and allocate everything
        as a single malloc block and suballocate internally.
     */
     if ((UINT_MAX - mainTableSize) / tupleIntSize < size) {
         nwrite = sprintf(message,
                          "size %u is too big for arithmetic",
                          size);
         if (nwrite > 0) {
             sixel_helper_set_additional_message(message);
         }
         status = SIXEL_RUNTIME_ERROR;
         goto end;
     }
 
     allocSize = mainTableSize + size * tupleIntSize;
 
     pool = sixel_allocator_malloc(allocator, allocSize);
     if (pool == NULL) {
         sprintf(message,
                 "unable to allocate %u bytes for a %u-entry "
                 "tuple table",
                  allocSize, size);
         sixel_helper_set_additional_message(message);
         status = SIXEL_BAD_ALLOCATION;
         goto end;
     }
     tbl = (tupletable) pool;
 
     for (i = 0; i < size; ++i)
         tbl[i] = (struct tupleint *)
             ((char*)pool + mainTableSize + i * tupleIntSize);
 
     *result = tbl;
 
     status = SIXEL_OK;
 
 end:
     return status;
 }
 
 
 /*
 ** Here is the fun part, the median-cut colormap generator.  This is based
 ** on Paul Heckbert's paper "Color Image Quantization for Frame Buffer
 ** Display", SIGGRAPH '82 Proceedings, page 297.
 */
 
 static tupletable2
 newColorMap(unsigned int const newcolors, unsigned int const depth, sixel_allocator_t *allocator)
 {
     SIXELSTATUS status = SIXEL_FALSE;
     tupletable2 colormap;
     unsigned int i;
 
     colormap.size = 0;
     status = alloctupletable(&colormap.table, depth, newcolors, allocator);
     if (SIXEL_FAILED(status)) {
         goto end;
     }
     if (colormap.table) {
         for (i = 0; i < newcolors; ++i) {
             unsigned int plane;
             for (plane = 0; plane < depth; ++plane)
                 colormap.table[i]->tuple[plane] = 0;
         }
         colormap.size = newcolors;
     }
 
 end:
     return colormap;
 }
 
 
 static boxVector
 newBoxVector(
     unsigned int const  /* in */ colors,
     unsigned int const  /* in */ sum,
     unsigned int const  /* in */ newcolors,
     sixel_allocator_t   /* in */ *allocator)
 {
     boxVector bv;
 
     bv = (boxVector)sixel_allocator_malloc(allocator,
                                            sizeof(struct box) * (size_t)newcolors);
     if (bv == NULL) {
         quant_trace(stderr, "out of memory allocating box vector table\n");
         return NULL;
     }
 
     /* Set up the initial box. */
     bv[0].ind = 0;
     bv[0].colors = colors;
     bv[0].sum = sum;
 
     return bv;
 }
 
 
 static void
 findBoxBoundaries(tupletable2  const colorfreqtable,
                   unsigned int const depth,
                   unsigned int const boxStart,
                   unsigned int const boxSize,
                   sample             minval[],
                   sample             maxval[])
 {
 /*----------------------------------------------------------------------------
   Go through the box finding the minimum and maximum of each
   component - the boundaries of the box.
 -----------------------------------------------------------------------------*/
     unsigned int plane;
     unsigned int i;
 
     for (plane = 0; plane < depth; ++plane) {
         minval[plane] = colorfreqtable.table[boxStart]->tuple[plane];
         maxval[plane] = minval[plane];
     }
 
     for (i = 1; i < boxSize; ++i) {
         for (plane = 0; plane < depth; ++plane) {
             sample const v = colorfreqtable.table[boxStart + i]->tuple[plane];
             if (v < minval[plane]) minval[plane] = v;
             if (v > maxval[plane]) maxval[plane] = v;
         }
     }
 }
 
 
 
 static unsigned int
 largestByNorm(sample minval[], sample maxval[], unsigned int const depth)
 {
 
     unsigned int largestDimension;
     unsigned int plane;
     sample largestSpreadSoFar;
 
     largestSpreadSoFar = 0;
     largestDimension = 0;
     for (plane = 0; plane < depth; ++plane) {
         sample const spread = maxval[plane]-minval[plane];
         if (spread > largestSpreadSoFar) {
             largestDimension = plane;
             largestSpreadSoFar = spread;
         }
     }
     return largestDimension;
 }
 
 
 
 static unsigned int
 largestByLuminosity(sample minval[], sample maxval[], unsigned int const depth)
 {
 /*----------------------------------------------------------------------------
    This subroutine presumes that the tuple type is either
    BLACKANDWHITE, GRAYSCALE, or RGB (which implies pamP->depth is 1 or 3).
    To save time, we don't actually check it.
 -----------------------------------------------------------------------------*/
     unsigned int retval;
 
     double lumin_factor[3] = {0.2989, 0.5866, 0.1145};
 
     if (depth == 1) {
         retval = 0;
     } else {
         /* An RGB tuple */
         unsigned int largestDimension;
         unsigned int plane;
         double largestSpreadSoFar;
 
         largestSpreadSoFar = 0.0;
         largestDimension = 0;
 
         for (plane = 0; plane < 3; ++plane) {
             double const spread =
                 lumin_factor[plane] * (maxval[plane]-minval[plane]);
             if (spread > largestSpreadSoFar) {
                 largestDimension = plane;
                 largestSpreadSoFar = spread;
             }
         }
         retval = largestDimension;
     }
     return retval;
 }
 
 
 
 static void
 centerBox(unsigned int const boxStart,
           unsigned int const boxSize,
           tupletable2  const colorfreqtable,
           unsigned int const depth,
           tuple        const newTuple)
 {
 
     unsigned int plane;
     sample minval, maxval;
     unsigned int i;
 
     for (plane = 0; plane < depth; ++plane) {
         minval = maxval = colorfreqtable.table[boxStart]->tuple[plane];
 
         for (i = 1; i < boxSize; ++i) {
             sample v = colorfreqtable.table[boxStart + i]->tuple[plane];
             minval = minval < v ? minval: v;
             maxval = maxval > v ? maxval: v;
         }
         newTuple[plane] = (minval + maxval) / 2;
     }
 }
 
 
 
 static void
 averageColors(unsigned int const boxStart,
               unsigned int const boxSize,
               tupletable2  const colorfreqtable,
               unsigned int const depth,
               tuple        const newTuple)
 {
     unsigned int plane;
     sample sum;
     unsigned int i;
 
     for (plane = 0; plane < depth; ++plane) {
         sum = 0;
 
         for (i = 0; i < boxSize; ++i) {
             sum += colorfreqtable.table[boxStart + i]->tuple[plane];
         }
 
         newTuple[plane] = sum / boxSize;
     }
 }
 
 
 
 static void
 averagePixels(unsigned int const boxStart,
               unsigned int const boxSize,
               tupletable2 const colorfreqtable,
               unsigned int const depth,
               tuple const newTuple)
 {
 
     unsigned int n;
         /* Number of tuples represented by the box */
     unsigned int plane;
     unsigned int i;
 
     /* Count the tuples in question */
     n = 0;  /* initial value */
     for (i = 0; i < boxSize; ++i) {
         n += (unsigned int)colorfreqtable.table[boxStart + i]->value;
     }
 
     for (plane = 0; plane < depth; ++plane) {
         sample sum;
 
         sum = 0;
 
         for (i = 0; i < boxSize; ++i) {
             sum += colorfreqtable.table[boxStart + i]->tuple[plane]
                 * (unsigned int)colorfreqtable.table[boxStart + i]->value;
         }
 
         newTuple[plane] = sum / n;
     }
 }
 
 
 
 static tupletable2
 colormapFromBv(unsigned int const newcolors,
                boxVector const bv,
                unsigned int const boxes,
                tupletable2 const colorfreqtable,
                unsigned int const depth,
                int const methodForRep,
                sixel_allocator_t *allocator)
 {
     /*
     ** Ok, we've got enough boxes.  Now choose a representative color for
     ** each box.  There are a number of possible ways to make this choice.
     ** One would be to choose the center of the box; this ignores any structure
     ** within the boxes.  Another method would be to average all the colors in
     ** the box - this is the method specified in Heckbert's paper.  A third
     ** method is to average all the pixels in the box.
     */
     tupletable2 colormap;
     unsigned int bi;
 
     colormap = newColorMap(newcolors, depth, allocator);
     if (!colormap.size) {
         return colormap;
     }
 
     for (bi = 0; bi < boxes; ++bi) {
         switch (methodForRep) {
         case SIXEL_REP_CENTER_BOX:
             centerBox(bv[bi].ind, bv[bi].colors,
                       colorfreqtable, depth,
                       colormap.table[bi]->tuple);
             break;
         case SIXEL_REP_AVERAGE_COLORS:
             averageColors(bv[bi].ind, bv[bi].colors,
                           colorfreqtable, depth,
                           colormap.table[bi]->tuple);
             break;
         case SIXEL_REP_AVERAGE_PIXELS:
             averagePixels(bv[bi].ind, bv[bi].colors,
                           colorfreqtable, depth,
                           colormap.table[bi]->tuple);
             break;
         default:
             quant_trace(stderr, "Internal error: "
                                 "invalid value of methodForRep: %d\n",
                         methodForRep);
         }
     }
     return colormap;
 }
 
 
 static SIXELSTATUS
 splitBox(boxVector const bv,
          unsigned int *const boxesP,
          unsigned int const bi,
          tupletable2 const colorfreqtable,
          unsigned int const depth,
          int const methodForLargest)
 {
 /*----------------------------------------------------------------------------
    Split Box 'bi' in the box vector bv (so that bv contains one more box
    than it did as input).  Split it so that each new box represents about
    half of the pixels in the distribution given by 'colorfreqtable' for
    the colors in the original box, but with distinct colors in each of the
    two new boxes.
 
    Assume the box contains at least two colors.
 -----------------------------------------------------------------------------*/
     SIXELSTATUS status = SIXEL_FALSE;
     unsigned int const boxStart = bv[bi].ind;
     unsigned int const boxSize  = bv[bi].colors;
     unsigned int const sm       = bv[bi].sum;
 
     enum { max_depth= 16 };
     sample minval[max_depth];
     sample maxval[max_depth];
 
     /* assert(max_depth >= depth); */
 
     unsigned int largestDimension;
         /* number of the plane with the largest spread */
     unsigned int medianIndex;
     unsigned int lowersum;
         /* Number of pixels whose value is "less than" the median */
 
     findBoxBoundaries(colorfreqtable, depth, boxStart, boxSize,
                       minval, maxval);
 
     /* Find the largest dimension, and sort by that component.  I have
        included two methods for determining the "largest" dimension;
        first by simply comparing the range in RGB space, and second by
        transforming into luminosities before the comparison.
     */
     switch (methodForLargest) {
     case SIXEL_LARGE_NORM:
         largestDimension = largestByNorm(minval, maxval, depth);
         break;
     case SIXEL_LARGE_LUM:
         largestDimension = largestByLuminosity(minval, maxval, depth);
         break;
     default:
         sixel_helper_set_additional_message(
             "Internal error: invalid value of methodForLargest.");
         status = SIXEL_LOGIC_ERROR;
         goto end;
     }
 
     /* TODO: I think this sort should go after creating a box,
        not before splitting.  Because you need the sort to use
        the SIXEL_REP_CENTER_BOX method of choosing a color to
        represent the final boxes
     */
 
     /* Set the gross global variable 'compareplanePlane' as a
        parameter to compareplane(), which is called by qsort().
     */
     compareplanePlane = largestDimension;
     qsort((char*) &colorfreqtable.table[boxStart], boxSize,
           sizeof(colorfreqtable.table[boxStart]),
           compareplane);
 
     {
         /* Now find the median based on the counts, so that about half
            the pixels (not colors, pixels) are in each subdivision.  */
 
         unsigned int i;
 
         lowersum = colorfreqtable.table[boxStart]->value; /* initial value */
         for (i = 1; i < boxSize - 1 && lowersum < sm / 2; ++i) {
             lowersum += colorfreqtable.table[boxStart + i]->value;
         }
         medianIndex = i;
     }
     /* Split the box, and sort to bring the biggest boxes to the top.  */
 
     bv[bi].colors = medianIndex;
     bv[bi].sum = lowersum;
     bv[*boxesP].ind = boxStart + medianIndex;
     bv[*boxesP].colors = boxSize - medianIndex;
     bv[*boxesP].sum = sm - lowersum;
     ++(*boxesP);
     qsort((char*) bv, *boxesP, sizeof(struct box), sumcompare);
 
     status = SIXEL_OK;
 
 end:
     return status;
 }
 
 
 
 static SIXELSTATUS
 mediancut(tupletable2 const colorfreqtable,
           unsigned int const depth,
           unsigned int const newcolors,
           int const methodForLargest,
           int const methodForRep,
           tupletable2 *const colormapP,
           sixel_allocator_t *allocator)
 {
 /*----------------------------------------------------------------------------
    Compute a set of only 'newcolors' colors that best represent an
    image whose pixels are summarized by the histogram
    'colorfreqtable'.  Each tuple in that table has depth 'depth'.
    colorfreqtable.table[i] tells the number of pixels in the subject image
    have a particular color.
 
    As a side effect, sort 'colorfreqtable'.
 -----------------------------------------------------------------------------*/
     boxVector bv;
     unsigned int bi;
     unsigned int boxes;
     int multicolorBoxesExist;
     unsigned int i;
     unsigned int sum;
     SIXELSTATUS status = SIXEL_FALSE;
 
     sum = 0;
 
     for (i = 0; i < colorfreqtable.size; ++i) {
         sum += colorfreqtable.table[i]->value;
     }
 
     /* There is at least one box that contains at least 2 colors; ergo,
        there is more splitting we can do.  */
     bv = newBoxVector(colorfreqtable.size, sum, newcolors, allocator);
     if (bv == NULL) {
         goto end;
     }
     boxes = 1;
     multicolorBoxesExist = (colorfreqtable.size > 1);
 
     /* Main loop: split boxes until we have enough. */
     while (boxes < newcolors && multicolorBoxesExist) {
         /* Find the first splittable box. */
         for (bi = 0; bi < boxes && bv[bi].colors < 2; ++bi)
             ;
         if (bi >= boxes) {
             multicolorBoxesExist = 0;
         } else {
             status = splitBox(bv, &boxes, bi,
                               colorfreqtable, depth,
                               methodForLargest);
             if (SIXEL_FAILED(status)) {
                 goto end;
             }
         }
     }
     *colormapP = colormapFromBv(newcolors, bv, boxes,
                                 colorfreqtable, depth,
                                 methodForRep, allocator);
 
     sixel_allocator_free(allocator, bv);
 
     status = SIXEL_OK;
 
 end:
     return status;
 }
   */
 
 pub fn
 computeHash(data: &[u8], i: usize, depth: i32) -> i32
 {
    let mut hash = 0;
    for n in 0..depth {
        hash |= (data[i + depth as usize - 1 - n as usize] as i32 >> 3) << (n * 5);
    }
    hash
 }

#[derive(Clone)]
pub struct Tuple {
    pub value: i32,
    pub tuple: Vec<i32>,
 }
 
 pub fn
 computeHistogram(data: &[u8],
    length: i32,
    depth: i32,
    qualityMode: Quality) ->SixelResult<HashMap<i32, Tuple>>
 {
    let (max_sample, mut step) = match qualityMode {
        Quality::LOW => (18383, length / depth / 18383 * depth),
        Quality::HIGH => (18383, length / depth / 18383 * depth),
        Quality::AUTO | 
        Quality::HIGHCOLOR |
        Quality::FULL => (4003079, length / depth / 4003079 * depth),
    };

     if length < max_sample * depth {
         step = 6 * depth;
     }
 
     if step <= 0 {
         step = depth;
     }
 
     let  mut  histogram = vec![0; 1 << (depth * 5)];

     let mut memory = vec![0; 1 << (depth * 5)];
     let  mut it = 0;
     let  mut refe = 0;
     let  mut refmap =  0;

     let mut i = 0; 
     while i < length {
         let bucket_index = computeHash(data, i as usize, 3) as usize;
         if histogram[bucket_index] == 0 {
             memory[refe] = bucket_index;
             refe+=1;
         }
         if histogram[bucket_index] < (1 << (2 * 8)) - 1 {
             histogram[bucket_index] += 1;
         }

         i += step;
     }
     let mut colorfreqtable = HashMap::new();
 
     for i in 0..refe {
         if histogram[memory[i]] > 0 {
            let mut tuple: Vec<i32> = vec![0; depth as usize];
            for n in 0..depth {
                tuple[(depth - 1 - n) as usize]
                     = ((memory[it] >> (n * 5) & 0x1f) << 3) as i32;
             }
             colorfreqtable.insert(i as i32,Tuple {
                value: histogram[memory[i]],
                tuple
            });
         }
         it += 1;
     }
     Ok(colorfreqtable)
 }
 

 pub fn
 computeColorMapFromInput(data: &[u8],
                            length:i32,
                          depth:i32,
                          reqColors:i32,
                          methodForLargest:FindLargestDim,
                          methodForRep:ColorChoosingMethod,
                          qualityMode:Quality,
                          colormapP: &mut HashMap<i32, Tuple>,
                          origcolors: &mut i32) ->SixelResult<()>
 {
 /*----------------------------------------------------------------------------
    Produce a colormap containing the best colors to represent the
    image stream in file 'ifP'.  Figure it out using the median cut
    technique.
 
    The colormap will have 'reqcolors' or fewer colors in it, unless
    'allcolors' is true, in which case it will have all the colors that
    are in the input.
 
    The colormap has the same maxval as the input.
 
    Put the colormap in newly allocated storage as a tupletable2
    and return its address as *colormapP.  Return the number of colors in
    it as *colorsP and its maxval as *colormapMaxvalP.
 
    Return the characteristics of the input file as
    *formatP and *freqPamP.  (This information is not really
    relevant to our colormap mission; just a fringe benefit).
 -----------------------------------------------------------------------------*/
 
    let mut colorfreqtable = computeHistogram(data, length, depth, qualityMode)?;
    *origcolors = colorfreqtable.len() as i32;
 
     if colorfreqtable.len() as i32 <= reqColors {
        for i in colorfreqtable.len() as i32..=reqColors {
            let mut tuple: Vec<i32> = vec![0; depth as usize];
            for n in 0..depth {
                tuple[n as usize] = (i * depth) + n;
             }
             colorfreqtable.insert(i, Tuple {
                value: i,
                tuple
            });
        }
         
        for i in 0..colorfreqtable.len() as i32 {
            colormapP.insert(i, colorfreqtable.get(&i).unwrap().clone());
         }
     } else {
        todo!("mediancut");
        /*/
         status = mediancut(colorfreqtable, depth, reqColors,
                            methodForLargest, methodForRep, colormapP, allocator);
         if (SIXEL_FAILED(status)) {
             goto end;
         }*/
     }
     Ok(())
 }

 /* diffuse error energy to surround pixels */
 pub fn
 error_diffuse(data:&mut [u8],  /* base address of pixel buffer */
    pos: i32,        /* address of the destination pixel */
    depth: i32,      /* color depth in bytes */
    error: i32,      /* error energy */
    numerator: i32,  /* numerator of diffusion coefficient */
    denominator: i32 /* denominator of diffusion coefficient */)
 {
     let offset= (pos * depth) as usize;
 
     let mut c = data[offset] as i32 + error * numerator / denominator;
     if c < 0 {
         c = 0;
     }
     if c >= 1 << 8 {
         c = (1 << 8) - 1;
     }
     data[offset] = c as u8;
 }
 
 
 pub fn
 diffuse_none(data:&mut [u8], width:i32, height:i32,
    x:i32, y:i32, depth:i32, error:i32)
 {
    
 }
 
 
 pub fn
 diffuse_fs(data:&mut [u8], width:i32, height:i32,
    x:i32, y:i32, depth:i32, error:i32)
 {
     let pos = y * width + x;
 
     /* Floyd Steinberg Method
      *          curr    7/16
      *  3/16    5/48    1/16
      */
     if x < width - 1 && y < height - 1 {
         /* add error to the right cell */
         error_diffuse(data, pos + width * 0 + 1, depth, error, 7, 16);
         /* add error to the left-bottom cell */
         error_diffuse(data, pos + width * 1 - 1, depth, error, 3, 16);
         /* add error to the bottom cell */
         error_diffuse(data, pos + width * 1 + 0, depth, error, 5, 16);
         /* add error to the right-bottom cell */
         error_diffuse(data, pos + width * 1 + 1, depth, error, 1, 16);
     }
 }
 
 
 pub fn
 diffuse_atkinson(data:&mut [u8], width:i32, height:i32,
    x:i32, y:i32, depth:i32, error:i32)
 {
     let pos = y * width + x;
 
     /* Atkinson's Method
      *          curr    1/8    1/8
      *   1/8     1/8    1/8
      *           1/8
      */
     if y < height - 2 {
         /* add error to the right cell */
         error_diffuse(data, pos + width * 0 + 1, depth, error, 1, 8);
         /* add error to the 2th right cell */
         error_diffuse(data, pos + width * 0 + 2, depth, error, 1, 8);
         /* add error to the left-bottom cell */
         error_diffuse(data, pos + width * 1 - 1, depth, error, 1, 8);
         /* add error to the bottom cell */
         error_diffuse(data, pos + width * 1 + 0, depth, error, 1, 8);
         /* add error to the right-bottom cell */
         error_diffuse(data, pos + width * 1 + 1, depth, error, 1, 8);
         /* add error to the 2th bottom cell */
         error_diffuse(data, pos + width * 2 + 0, depth, error, 1, 8);
     }
 }
 
 
 pub fn
 diffuse_jajuni(data:&mut [u8], width:i32, height:i32,
    x:i32, y:i32, depth:i32, error:i32)
 {
     let pos = y * width + x;
 
     /* Jarvis, Judice & Ninke Method
      *                  curr    7/48    5/48
      *  3/48    5/48    7/48    5/48    3/48
      *  1/48    3/48    5/48    3/48    1/48
      */
     if pos < (height - 2) * width - 2 {
         error_diffuse(data, pos + width * 0 + 1, depth, error, 7, 48);
         error_diffuse(data, pos + width * 0 + 2, depth, error, 5, 48);
         error_diffuse(data, pos + width * 1 - 2, depth, error, 3, 48);
         error_diffuse(data, pos + width * 1 - 1, depth, error, 5, 48);
         error_diffuse(data, pos + width * 1 + 0, depth, error, 7, 48);
         error_diffuse(data, pos + width * 1 + 1, depth, error, 5, 48);
         error_diffuse(data, pos + width * 1 + 2, depth, error, 3, 48);
         error_diffuse(data, pos + width * 2 - 2, depth, error, 1, 48);
         error_diffuse(data, pos + width * 2 - 1, depth, error, 3, 48);
         error_diffuse(data, pos + width * 2 + 0, depth, error, 5, 48);
         error_diffuse(data, pos + width * 2 + 1, depth, error, 3, 48);
         error_diffuse(data, pos + width * 2 + 2, depth, error, 1, 48);
     }
 }
 
 
 pub fn
 diffuse_stucki(data:&mut [u8], width:i32, height:i32,
    x:i32, y:i32, depth:i32, error:i32)
 {
     let pos = y * width + x;
 
     /* Stucki's Method
      *                  curr    8/48    4/48
      *  2/48    4/48    8/48    4/48    2/48
      *  1/48    2/48    4/48    2/48    1/48
      */
     if pos < (height - 2) * width - 2 {
         error_diffuse(data, pos + width * 0 + 1, depth, error, 1, 6);
         error_diffuse(data, pos + width * 0 + 2, depth, error, 1, 12);
         error_diffuse(data, pos + width * 1 - 2, depth, error, 1, 24);
         error_diffuse(data, pos + width * 1 - 1, depth, error, 1, 12);
         error_diffuse(data, pos + width * 1 + 0, depth, error, 1, 6);
         error_diffuse(data, pos + width * 1 + 1, depth, error, 1, 12);
         error_diffuse(data, pos + width * 1 + 2, depth, error, 1, 24);
         error_diffuse(data, pos + width * 2 - 2, depth, error, 1, 48);
         error_diffuse(data, pos + width * 2 - 1, depth, error, 1, 24);
         error_diffuse(data, pos + width * 2 + 0, depth, error, 1, 12);
         error_diffuse(data, pos + width * 2 + 1, depth, error, 1, 24);
         error_diffuse(data, pos + width * 2 + 2, depth, error, 1, 48);
     }
 }
 
 
 pub fn
 diffuse_burkes(data:&mut [u8], width:i32, height:i32,
    x:i32, y:i32, depth:i32, error:i32)
 {
     let pos = y * width + x;
 
     /* Burkes' Method
      *                  curr    4/16    2/16
      *  1/16    2/16    4/16    2/16    1/16
      */
     if pos < (height - 1) * width - 2 {
         error_diffuse(data, pos + width * 0 + 1, depth, error, 1, 4);
         error_diffuse(data, pos + width * 0 + 2, depth, error, 1, 8);
         error_diffuse(data, pos + width * 1 - 2, depth, error, 1, 16);
         error_diffuse(data, pos + width * 1 - 1, depth, error, 1, 8);
         error_diffuse(data, pos + width * 1 + 0, depth, error, 1, 4);
         error_diffuse(data, pos + width * 1 + 1, depth, error, 1, 8);
         error_diffuse(data, pos + width * 1 + 2, depth, error, 1, 16);
     }
 }
 
 pub fn
 mask_a (x:i32, y:i32, c:i32) -> f32
 {
     return ((((x + c * 67) + y * 236) * 119) & 255 ) as f32 / 128.0 - 1.0;
 }
 
 pub fn
 mask_x (x:i32, y:i32, c:i32) -> f32
 {
     return ((((x + c * 29) ^ y * 149) * 1234) & 511 ) as f32 / 256.0 - 1.0;
 }

use std::{collections::HashMap, hash::Hash};

use crate::{ColorChoosingMethod, SixelError, DiffusionMethod};
use crate::{SixelResult, pixelformat::sixel_helper_compute_depth, FindLargestDim, ResampleMethod, PixelFormat, Quality};


 /* lookup closest color from palette with "normal" strategy */
pub fn
 lookup_normal(pixel: &[u8],
    depth: i32,
    palette: &[u8],
    reqcolor: i32,
    cachetable:   &mut Vec<u16>,
    complexion: i32) -> i32
 {
  
     let mut result = -1;
     let mut diff = i32::MAX;

     /* don't use cachetable in 'normal' strategy */
     
     for i in 0..reqcolor {
         let mut distant = 0;
         let mut r = pixel[0] as i32 - palette[(i * depth + 0) as usize] as i32;
         distant += r * r * complexion;
         for n in 1..depth {
             r = pixel[n as usize] as i32 - palette[(i * depth + n) as usize] as i32;
             distant += r * r;
         }
         if distant < diff {
             diff = distant;
             result = i;
         }
     }
 
     return result;
 }
 
 /* lookup closest color from palette with "fast" strategy */
 pub fn
 lookup_fast(pixel: &[u8],
    depth: i32,
    palette: &[u8],
    reqcolor: i32,
    cachetable:   &mut Vec<u16>,
    complexion: i32) -> i32
 {
    let mut result: i32 = -1;
    let mut diff = i32::MAX;
    let mut hash = computeHash(pixel, 0, 3);
 
    let cache = cachetable[hash as usize];
     if cache != 0 {  /* fast lookup */
         return cache as i32 - 1;
     }
     /* collision */
     for i in 0..reqcolor {
/*          distant = 0;
  #if 0
         for (n = 0; n < 3; ++n) {
             r = pixel[n] - palette[i * 3 + n];
             distant += r * r;
         }
 #elif 1*/  /* complexion correction */
         let i = i as usize;
         let distant = 
                   (pixel[0] as i32 - palette[i * 3 + 0] as i32) * (pixel[0] as i32 - palette[i * 3 + 0] as i32) * complexion
                 + (pixel[1] as i32 - palette[i * 3 + 1] as i32) * (pixel[1] as i32 - palette[i * 3 + 1] as i32)
                 + (pixel[2] as i32 - palette[i * 3 + 2] as i32) * (pixel[2] as i32 - palette[i * 3 + 2] as i32)
                 ;
//  #endif
         if distant < diff {
             diff = distant;
             result = i as i32;
         }
     }
     cachetable[hash as usize] = (result + 1) as u16;
 
    result
 }

 
 pub fn
 lookup_mono_darkbg(pixel: &[u8],
    depth: i32,
    palette: &[u8],
    reqcolor: i32,
    cachetable:   &mut Vec<u16>,
    complexion: i32) -> i32
 {
    let mut distant = 0;
    for n in 0..depth {
        distant += pixel[n as usize] as i32;
    }
    if distant >= 128 * reqcolor { 1 }else { 0}
}
 
 pub fn
 lookup_mono_lightbg(pixel: &[u8],
    depth: i32,
    palette: &[u8],
    reqcolor: i32,
    cachetable:   &mut Vec<u16>,
    complexion: i32) -> i32
 {
    let mut distant = 0;
    for n in 0..depth {
        distant += pixel[n as usize] as i32;
    }
    if distant < 128 * reqcolor { 1 }else { 0}
}
 

 /* choose colors using median-cut method */
 pub fn
 sixel_quant_make_palette(
     data: &[u8]   ,
     length: i32,
     pixelformat: PixelFormat,
     reqcolors: i32,
     ncolors: &mut i32,
     origcolors: &mut i32,
     methodForLargest: FindLargestDim,
     methodForRep: ColorChoosingMethod,
     qualityMode: Quality) -> SixelResult<Vec<u8>>
 {
     let result_depth = sixel_helper_compute_depth(pixelformat);
     /*if (result_depth <= 0) {
         *result = NULL;
         goto end;
     }*/
 
     let depth =  result_depth as usize;
     let mut colormap = HashMap::new();
    computeColorMapFromInput(
        data, length, depth as i32,
                                    reqcolors, methodForLargest,
                                    methodForRep, qualityMode,
                                    &mut colormap, origcolors);
     *ncolors = *origcolors;
     let mut result = vec![0; colormap.len() * depth as usize];
     for i in 0..colormap.len() {
        for n in 0..depth {
             result[i * depth + n] = colormap.get(&(i as i32)).unwrap().tuple[n] as u8;
         }
     }
     Ok(result)
 }
 


 /* apply color palette into specified pixel buffers */
 pub fn
 sixel_quant_apply_palette(
    result: &mut [u8],
    data: &mut [u8],
    width: i32,
    height: i32,
    depth: i32,
    palette: &mut Vec<u8>,
    reqcolor:i32,
    methodForDiffuse: DiffusionMethod,
    foptimize: bool,
    foptimize_palette: bool,
    complexion: i32,
    cachetable: Option<&mut Vec<u16>>) -> SixelResult<i32>
 {
    let mut ncolors: i32 = 0;
     /* check bad reqcolor */
     if reqcolor < 1 {
        /*
                 sixel_helper_set_additional_message(
             "sixel_quant_apply_palette: "
             "a bad argument is detected, reqcolor < 0.");
         */
        return Err(Box::new(SixelError::BadArgument));
     }

    let mut f_mask = false;


     let f_diffuse = if depth != 3 {
        diffuse_none
     } else {
         match methodForDiffuse {
            DiffusionMethod::Auto |
            DiffusionMethod::None => diffuse_none,
            DiffusionMethod::Atkinson => diffuse_atkinson,
            DiffusionMethod::FS => diffuse_fs,
            DiffusionMethod::JaJuNi => diffuse_jajuni,
            DiffusionMethod::Stucki => diffuse_stucki,
            DiffusionMethod::Burkes => diffuse_burkes,
            DiffusionMethod::ADither => {
                f_mask = true;
                diffuse_none
            }
            DiffusionMethod::XDither => {
                f_mask = true;
                diffuse_none    
            }
        }
     };
 
     let mut f_lookup: Option<fn(&[u8], i32, &[u8], i32, &mut Vec<u16>, i32) -> i32> = None;
     if reqcolor == 2 {
         let mut sum1 = 0;
         let mut sum2 = 0;
         for n in 0..depth {
             sum1 += palette[n as usize] as i32;
         }
         for n  in depth..(depth + depth) {
             sum2 += palette[n as usize] as i32;
         }
         if (sum1 == 0 && sum2 == 255 * 3) {
             f_lookup = Some(lookup_mono_darkbg);
         } else if (sum1 == 255 * 3 && sum2 == 0) {
             f_lookup = Some(lookup_mono_lightbg);
         }
     }
     if f_lookup.is_none() {
         if (foptimize && depth == 3) {
             f_lookup = Some(lookup_fast);
         } else {
             f_lookup = Some(lookup_normal);
         }
     }
 
     let mut cc = vec![0u16, 1 << (depth * 5)];
     let mut indextable = match cachetable {
            Some(table) => table,
            None => &mut cc,
     };
 
     if foptimize_palette {
         ncolors = 0;

         let mut new_palette = Vec::new();
         let mut migration_map = Vec::new();
 
         if f_mask {
            for y in 0..height {
                for x in 0..width {
                    let mut copy: Vec<u8> = Vec::new();
 
                    let pos = y * width + x;
                     for d in 0..depth {
                         let mut val = data[(pos * depth + d) as usize] as i32;
                         if matches!(methodForDiffuse, DiffusionMethod::ADither) {
                            val += (mask_a(x, y, d) * 32.0) as i32;
                        } else {
                           val += (mask_x(x, y, d) * 32.0) as i32;
                        }
                        copy.push (val.clamp(0, 255) as u8);
                     }
  //                   &[u8], i32, &[u8], i32, &mut Vec<u16>, i32
                     let color_index = f_lookup.unwrap()(&copy, 
                        depth,
                                            &palette, reqcolor, &mut indextable, complexion) as usize;
                     if migration_map[color_index] == 0 {
                         result[pos as usize] = ncolors as u8;
                         for n  in 0..depth {
                            new_palette.push(palette[color_index * depth as usize + n as usize]);
                         }
                         ncolors += 1;
                         migration_map[color_index] = ncolors;
                     } else {
                         result[pos as usize] = migration_map[color_index] as u8 - 1;
                     }
                 }
             }
             *palette = new_palette;
         } else {
            for y in 0..height {
                for x in 0..width {
                    let pos = y * width + x;
                    let color_index = f_lookup.unwrap()(&data[(pos * depth) as usize..], depth,
                                            palette, reqcolor, &mut indextable, complexion) as usize;
                     if (migration_map[color_index] == 0) {
                         result[pos as usize] = ncolors as u8;
                         for n  in 0..depth {
                            new_palette[(ncolors * depth + n) as usize] = palette[(color_index * depth as usize + n as usize) as usize];
                         }
                         ncolors += 1;
                         migration_map[color_index] = ncolors;
                     } else {
                         result[pos as usize] = migration_map[color_index] as u8 - 1;
                     }
                     for n  in 0..depth {
                        let offset = data[(pos * depth + n)as usize] as i32 - palette[color_index * depth as usize + n as usize] as i32;
                        f_diffuse(&mut data[n as usize..], width, height, x, y, depth, offset);
                     }
                 }
             }
             *palette = new_palette;
         }
     } else {
         if (f_mask) {
            for y in 0..height {
                for x in 0..width {
                    let mut copy: Vec<u8> = Vec::new();
                     let pos = y * width + x;
                     for d in 0..depth {
                        let mut val = data[(pos * depth + d) as usize] as i32;
                        if matches!(methodForDiffuse, DiffusionMethod::ADither) {
                            val += (mask_a(x, y, d) * 32.0) as i32;
                        } else {
                           val += (mask_x(x, y, d) * 32.0) as i32;
                        }

                         copy.push(val.clamp(0, 255) as u8);
                     }
                     result[pos as usize] = f_lookup.unwrap()(&mut copy, depth,
                                            palette, reqcolor, &mut indextable, complexion) as u8;
                 }
             }
         } else {
            for y in 0..height {
                for x in 0..width {
                    let pos = y * width + x;
                    let color_index = f_lookup.unwrap()(&mut data[(pos * depth) as usize..], depth,
                                            palette, reqcolor, &mut indextable, complexion) as usize;
                    result[pos as usize] = color_index as u8;
                     for n  in 0..depth {
                        let offset = data[(pos * depth + n) as usize] as i32 - palette[color_index * depth as usize + n as usize] as i32;
                         f_diffuse(&mut data[n as usize..], width, height, x, y, depth, offset);
                     }
                 }
             }
         }
         ncolors = reqcolor;
     }

     Ok(ncolors)
 }

 /* emacs Local Variables:      */
 /* emacs mode: c               */
 /* emacs tab-width: 4          */
 /* emacs indent-tabs-mode: nil */
 /* emacs c-basic-offset: 4     */
 /* emacs End:                  */
 /* vim: set expandtab ts=4 sts=4 sw=4 : */
 /* EOF */

 /*
 *
 * mediancut algorithm implementation is imported from pnmcolormap.c
 * in netpbm library.
 * http://netpbm.sourceforge.net/
 *
 * *******************************************************************************
 *                  original license block of pnmcolormap.c
 * *******************************************************************************
 *
 *   Derived from ppmquant, originally by Jef Poskanzer.
 *
 *   Copyright (C) 1989, 1991 by Jef Poskanzer.
 *   Copyright (C) 2001 by Bryan Henderson.
 *
 *   Permission to use, copy, modify, and distribute this software and its
 *   documentation for any purpose and without fee is hereby granted, provided
 *   that the above copyright notice appear in all copies and that both that
 *   copyright notice and this permission notice appear in supporting
 *   documentation.  This software is provided "as is" without express or
 *   implied warranty.
 *
 * ******************************************************************************
 *
 * Copyright (c) 2014-2018 Hayaki Saito
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 *
 *
 */